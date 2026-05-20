---
slug: chart-x-axis-local-time
status: shipped
owner: operator
updated: 2026-05-20
version: 1.11.0
predecessor: chart-canvas-overhaul v1.10.0
---

# Chart x-axis local time (`chart-x-axis-local-time`) — v1.11

> Closes the operator-friendly local-time landing deferred from
> [`chart-canvas-overhaul v1.10.0`](../chart-canvas-overhaul/feature.md)
> by operator-locked **Q-revised-1 = path (b)**. The deferral was
> intentional — v1.10.0 ships UTC x-axis labels; v1.11 owns the
> workspace `time` `local-offset` feature flip plus the production-OS-
> offset wiring.

## Why

The cockpit's chart bottom-axis renders `HH:MM` labels via
[`local_offset_or_utc()`](../../crates/ui/src/widgets/chart.rs#L175)
at `crates/ui/src/widgets/chart.rs:175-182`. v1.10.0 shipped with a
deterministic `UtcOffset::UTC` return in both production AND test
because the `time` crate's `local-offset` feature was off at the
workspace `Cargo.toml`. The deferral preserved snapshot determinism
through M7 while v1.11 owns the operator-facing local-time landing.

The deferral comment in `chart.rs:175-182` and the predecessor brief's
`## Design — M7 / Q4 deferral` both pre-anticipate the v1.11 flip:

> The function signature pre-anticipates the production-OS-offset
> branch so the v1.11 implementation flips only the body, not the
> call sites. The `cfg(test)` UTC override contract holds across
> the v1.11 cutover.

## Scope (operator-locked from predecessor M7 architect pass)

1. **Workspace `time` `local-offset` feature flip.** One-line
   `Cargo.toml` edit: add `local-offset` to the `time` crate's
   features array.
2. **Wire production-OS-offset in `local_offset_or_utc()`.**
   `crates/ui/src/widgets/chart.rs:175-182` — flip the body to call
   `time::UtcOffset::current_local_offset()` in production. Preserve
   `cfg(test)` UTC override deterministically.
3. **One unit test:** `local_offset_under_production_reads_os_offset`
   asserting the helper compiles under production (the `cfg(test)`
   path can't *itself* verify production behaviour, so the test
   asserts the production function signature returns `UtcOffset` and
   doesn't panic; the body-flip is mechanical and covered by the
   compiler's type check + the `time` crate's behavioural contract).

## Out of scope

- Linux / Windows support — operator-locked macOS-only cockpit at
  v1.10.0 + v1.11. The multi-threaded glibc deadlock caveat
  documented in the `time` crate's `local-offset` docs does not bite
  on macOS. Linux support is `cockpit-cross-platform` (queue
  candidate).
- Timezone display (TZ database lookup) — `UtcOffset` is sufficient
  for `HH:MM` axis labels; full TZ rendering is a separate v2
  feature.
- Cockpit chrome / status-strip clock — `local_offset_or_utc()` is
  chart-axis-only; the status strip already uses UTC and has no
  v1.11 ask.
- Snapshot-baseline refresh — the `cfg(test)` UTC override preserves
  all existing render-snapshot + visual_snapshot baselines
  byte-identically. No baseline refresh expected.

## Non-regression contract

1. 22 body-SHA-256 anchors stay byte-identical (R10.1; no
   strategy / audit / exec / report path is touched by this flip).
2. `cockpit-smoke` PASS (0 panic lines in 8 s window).
3. All existing render_snapshots + visual_snapshots stay green
   (`cfg(test)` UTC override is preserved).
4. `spec-lint` Phase contribution = 0.
5. No new external crate deps; no new Lumen tokens.
6. `cargo clippy --workspace -- -D warnings` exit 0.

## Trace

Trace row `REQ-CHART-X-AXIS-LOCAL-TIME-001` opened proposed → in-
progress.

## Changelog

- 2026-05-20 (orchestrator, direct ship): v1.11 promoted from
  `candidate` to `in-progress`. Scope is operator-locked from the
  predecessor's M7 architect pass; no analyst/architect cycle
  needed. Direct-edit pattern per CLAUDE.md ("Trivial → direct edit,
  run `rust-build` + `rust-validate` yourself"). The change is a
  one-line `Cargo.toml` flip + ~5-line body swap + 1 unit test.
