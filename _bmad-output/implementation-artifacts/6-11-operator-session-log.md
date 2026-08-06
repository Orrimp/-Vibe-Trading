# Story 6.11: operator-session-log

Status: ready-for-dev

<!-- Created 2026-08-04 by the adversarial product review (PRD §13 Q6; operator
     decision 2026-08-04: BUILD). Disclosure context: bug-log #66 A.4 — a shipped,
     "done" feature (Compare's report scanner) was silently broken for weeks and
     only a code review found it, because nothing records what the app DOES. -->

## Story

As the operator, the only user this product has,
I want a local, inspectable record of what the application actually did during a session,
so that "it works" stops resting entirely on tests — which have themselves been wrong five times (bug-log #65-#69).

## Acceptance Criteria

1. **It records what happened, not what was logged.** Per session: which screens were
   opened, which runs were started and with what inputs (strategy/pair/range/source),
   whether each produced a result or an error, and which reports/plans were written. The
   unit is the *operator-visible action and its outcome*, not a tracing firehose.
2. **Local, plain, and inspectable.** One append-only file per session under a git-ignored
   state dir, in a format readable without tooling (JSONL). No network, no telemetry
   service, no third party — this product's promise is offline honesty and this story must
   not dent it.
3. **It would have caught #66 A.4.** The acceptance test for this story is a replay of that
   defect's shape: a surface that renders but populates from nothing must show up in the
   log as "opened Compare → 0 cells from N discovered reports", i.e. the log records
   *emptiness with a denominator*, not just "screen opened". A log that cannot make that
   distinction has not met this AC.
4. **Zero cost when off, honest when on.** Off by default or trivially disabled; when on,
   no measurable impact on the render loop (the cockpit's input-responsiveness work is not
   to be regressed) and no money-path or gate behaviour touched.
5. **Retention is bounded and stated.** Sessions roll over; the cap is documented next to
   the file, alongside a one-line statement of exactly what is and is not recorded.
6. Standing floor: anchors 119/119; spec-lint PASS; no `println!` in library code; UI
   additions (if any) verified at the render layer per AD-10.

## Tasks / Subtasks

- [ ] Decide the event vocabulary (the "operator-visible action + outcome + denominator" shape) — small and closed, not extensible-by-accident.
- [ ] Dev: writer behind a trait (external I/O rule), session rollover, the off-switch.
- [ ] The #66-replay acceptance test (AC3) — the story's real gate.
- [ ] Document what is recorded, where, and for how long.

## Dev Notes

- Origin: product review 2026-08-04 finding 6 / PRD §13 Q6, operator decision BUILD.
- **The point is not observability for its own sake.** Five defects in this repo's bug log
  shipped past green tests; the common thread is that nothing independent of the tests
  recorded what the software actually did. This log is that independent record, at
  operator scale (one user, one machine) — not a metrics platform.
- Deliberately NOT in scope: remote telemetry, analytics, crash reporting, user
  identification, any network call whatsoever.
- Related: `crates/audit/` already durably records the *money* side (double-entry, SQLite).
  This story covers the *application behaviour* side. Reuse the audit crate's storage
  conventions where they fit; do not entangle the two schemas.

### References

- Trace: `REQ-OPERATOR-SESSION-LOG-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 6 (Remediation, Infra & Governance)
