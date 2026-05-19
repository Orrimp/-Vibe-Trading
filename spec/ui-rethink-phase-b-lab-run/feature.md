---
slug: ui-rethink-phase-b-lab-run
status: proposed
owner: pending-analyst
updated: 2026-05-19
version: 0.1.0
predecessor: ui-rethink-phase-a-lab v0.2.0
---

# UI rethink Phase B — Lab Run button (`ui-rethink-phase-b-lab-run`)

> This brief is the **second concrete feature** carved out of the broader
> UI rethink at
> [`spec/dev-notes/ui-rethink-2026-05-17.md`](../dev-notes/ui-rethink-2026-05-17.md).
> The dev-note's §6 Phase B is the spec source of truth; this brief is
> the **implementation contract** for that slice. Predecessor:
> [`ui-rethink-phase-a-lab v0.2.0`](../ui-rethink-phase-a-lab/feature.md)
> shipped 2026-05-18 — the Lab vertical (chart + chip widgets + tuple
> persistence + `Run` button that reads pre-computed reports from
> `spec/<strategy>/reports/`) lands as the foundation; Phase B promotes
> the `Run` button from "read cached report" to "actually run a backtest
> in-process."

## Why

Phase A shipped a Lab vertical where the `Run` button reads a pre-
computed report from `spec/<strategy>/reports/` if one matches the
tuple, otherwise renders a CLI hint ("run `cargo run --bin backtest
--strategy v1.momentum --pair ETHUSDT --range last-90d` then refresh").
This deferred two questions:

1. **Library-callable backend** — `crates/backtest` may still be
   shaped as a binary-first crate; Phase A intentionally read the
   resulting report files rather than calling the engine, so the
   binary-first assumption was never tested.
2. **End-to-end Lab vertical** — the operator's J2 workflow ("test
   a strategy against this pair AND this date range, see how
   successful the selection is") completes only when the Run button
   produces a fresh result; the CLI-hint shortcut is friction.

Phase B closes both gaps in one slice.

## Scope (dev-note §6 Phase B)

- **Backend cross-cut:** confirm `crates/backtest` is library-callable;
  refactor the binary into a thin wrapper over a library entry point
  if not.
- **Wire the Lab `Run` button** to call the engine and populate
  `lab_state.result` directly (instead of reading from
  `spec/<strategy>/reports/`).
- **Add the "compare to previous run" affordance** (dev-note §6 Phase
  B bullet 3) — diff the current `lab_state.result` against the
  pre-Run state to highlight P&L delta / drawdown delta.

## Out of scope

- Phase C (sidebar IA flip), Phase D (Trail), Phase E (Compare matrix),
  Phase F (Memory + Models + Assistant slot). Each is its own brief.
- New backtest engine internals — Phase B is wiring + minimal refactor,
  not engine work.
- Multi-strategy / multi-pair batch runs — that's Phase E (Compare).
- Live (paper-trading) mode — separate, gated on v2 LLM strategy.

## Analyst pass — open questions to surface

The analyst's first pass should turn this brief into a complete
feature.md with R1..Rn requirements + Q1..Qn operator decides. Likely
Qs:

- **Q1:** library-call shape — does `crates/backtest::run(...)` return
  the report struct in memory, or write to `spec/<strategy>/reports/`
  and have the Lab read back? (Memory is faster; disk is auditable.)
- **Q2:** spinner / progress UX while the engine runs. Reuse the
  shipped `throttled_spinner` (60→10 fps from
  `cockpit-performance-and-input-responsiveness v1.0.0`) or render a
  progress bar over (bars_processed / bars_total)?
- **Q3:** cancellation — does the Lab `Run` button become a Run/Cancel
  toggle mirroring the `lab::trainer` / `lab::runner` pattern, or is
  the in-process backtest synchronous-blocking with no cancel?
- **Q4:** the "compare to previous run" affordance — diff against (a)
  the last in-memory result of the same Lab session, (b) the most
  recent on-disk report matching the tuple, or (c) both with operator
  toggle?
- **Q5:** anchor risk surface — if the new library-call path produces
  a report-bytes-identical output to the CLI wrapper, anchors stay
  green. Confirm `crates/backtest` emits identical report bytes from
  both entry points; if not, that's a v2 anchor refresh task.

## Non-regression contract (placeholder — analyst to refine)

- 22 body-SHA-256 anchors stay byte-identical (15 originals + 4
  `-realdata` + 3 from v25-tcn-alpha-investigation).
- Phase A's Lab tuple persistence, chart + chip widgets, equity-curve
  overlay, comparison overlay — all stay byte-identical.
- `cockpit-smoke` stays green.
- `cockpit-performance-and-input-responsiveness v1.0.0`'s idle-CPU
  floor (≤13.1%) stays under budget after the engine integrates — a
  long-running in-process backtest is allowed to spike CPU during the
  run, but idle return-to-baseline is required.

## Trace

Trace row `REQ-UI-RETHINK-PHASE-B-001` to be opened in proposed state
by analyst pass.

## Changelog

- 2026-05-19 (orchestrator): brief stub opened on operator direction
  "1 then 2" (perf was already shipped; Phase B is the live next item).
  Predecessor verified at `ui-rethink-phase-a-lab v0.2.0` shipped
  2026-05-18. Status `proposed`; awaiting analyst pass.
