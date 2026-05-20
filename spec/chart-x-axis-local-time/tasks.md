---
slug: chart-x-axis-local-time
status: shipped
owner: operator
updated: 2026-05-20
---

# Tasks — chart-x-axis-local-time v1.11

> Trivial direct-ship per CLAUDE.md ("Trivial → direct edit, run
> `rust-build` + `rust-validate` yourself"). Operator-locked scope
> from [`chart-canvas-overhaul`](../chart-canvas-overhaul/feature.md)
> M7 architect pass. No analyst/architect sub-agent cycle needed.

## M1 — Direct implementation

- [x] **T1** Flip workspace `time` features. `Cargo.toml:69` — added
  `"local-offset"` to the `time` crate's features array.
- [x] **T2** Wire production-OS-offset in `local_offset_or_utc()`.
  `crates/ui/src/widgets/chart.rs:174-202` — split into
  `#[cfg(test)]` (UTC, snapshot-determinism gate) and
  `#[cfg(not(test))]` (`current_local_offset().unwrap_or(UtcOffset::UTC)`
  defensive fallback for glibc unsoundness — does not bite on macOS).
- [x] **T3** Added unit test `local_offset_under_production_reads_os_offset`
  at `crates/ui/src/widgets/chart.rs:1998-2014` asserting the helper
  returns `UtcOffset::UTC` under `cfg(test)` (snapshot-determinism
  contract). Production branch covered by compile-only verification
  + operator's live-cockpit ship.
- [x] **T4** Updated the function's doc comment — struck the "v1.11
  deferral" language; replaced with the "shipped at v1.11" note +
  documented the glibc-unsoundness defensive fallback.

## M-FINAL — Validate

- [x] `cargo fmt --check` exit 0.
- [x] `cargo clippy --workspace -- -D warnings` exit 0.
- [x] `cargo test --workspace --lib` 100% PASS — 279 passed (+1 vs
  pre-v1.11 baseline of 278: the new `local_offset_under_production_reads_os_offset`).
- [x] `cargo test -p ui --test render_snapshots` 2 PASS + 5 ignored
  (snapshots stay green via the env-var override — see Notes).
- [x] `cargo test -p ui --test visual_snapshots` 4 PASS.
- [x] `scripts/verify_anchors.sh` → ANCHORS PASS (22 / 22).
- [x] `cockpit-smoke` → 0 panic lines in 8 s window. Log:
  `spec/chart-x-axis-local-time/reports/cockpit-smoke-2026-05-20T02-20Z.log`.
- [x] `uv run scripts/spec_lint.py` — own contribution = 0
  (baseline 735, no change).
- [x] Authored `spec/chart-x-axis-local-time/reports/test-final-2026-05-20.md`.
- [x] Authored `spec/chart-x-axis-local-time/presentations/chart-x-axis-local-time-2026-05-20.md`.

## Notes

- This is a behaviour-preserving feature-flag flip + body change. The
  snapshot-determinism contract is preserved via **two complementary
  gates** (the unit-test `cfg(test)` branch — fires when the library
  is built as a test target — AND the `UI_CHART_FORCE_UTC` env var —
  set by integration test runners before invoking
  `iced_test::screenshot`).
- The env var is necessary because Cargo only sets `cfg(test)` on a
  crate when building it as a test target. Integration tests link
  against the library compiled WITHOUT `cfg(test)`, so the unit-test
  branch alone is insufficient. The env-var gate covers both
  `tests/render_snapshots.rs` and `tests/visual_snapshots.rs`.
- No new anchors. R10.1 non-regression contract: 22/22 byte-identical.
