# Story 6.11: operator-session-log

Status: done

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

- [x] Event vocabulary — **done 2026-09-24**: six events, and a CLOSED `Rejected` enum of seven reasons (ADR-0096 D1/D3).
- [x] Dev: `SessionSink` trait + `JsonlSink`, rollover, off-switch — **done 2026-09-24**.
- [x] The #66-replay acceptance test (AC3) — **done 2026-09-24**, `crates/ui/tests/session_log_replays_66_a4.rs`.
- [x] Document what is recorded, where, and for how long — **done 2026-09-24**, [`docs/runbooks/operator-session-log.md`](../../docs/runbooks/operator-session-log.md) **and** a README written beside the logs themselves.

## Dev record (2026-09-24)

ADR-0096.

| AC | What shipped |
|---|---|
| AC1 | Six events, each with a producer: `session_started`, `screen_opened` (on `SwitchScreen`), `run_started`/`run_finished` (the Lab pair, carrying strategy/pair/range/source — the same four the persistence schema records, so the log cannot describe a run by different inputs than the product saves), `scanned`, `artifact_written` (a saved forward-plan export). |
| AC2 | One append-only JSONL per session under `$XDG_STATE_HOME/trading/sessions/`, readable with `cat`. No network call of any kind. The directory is OUTSIDE the repo, so there is no `.gitignore` rule to forget. |
| AC3 | `ScanTally { discovered, admitted, rejected }` on `scan_report_roots`, and a **discrimination** test: the defect corpus and an empty corpus must record DIFFERENTLY. A presence test would pass on the very log that missed #66 A.4. |
| AC4 | `Option<Arc<dyn SessionSink>>` on the `Cockpit` — off is structural, `None` in every fixture/gallery/test, `TRADING_SESSION_LOG=0` for the binaries. An unopenable file costs one `warn!` and the cockpit runs unlogged. |
| AC5 | 30 sessions, pruned at boot, stated in the runbook AND in a README written beside the logs. |
| AC6 | anchors 119/119; spec-lint PASS; no `println!` in library code; no UI surface added, so AD-10 does not bind. |

### One honest note on the replay (AC3)

**#66 A.4's literal trigger no longer reproduces.** Its fix was a tolerant-reader change to
`parse_frontmatter`, which now accepts the unindented `strategy:` shape — verified while
writing the test, because the first version of the defect corpus was admitted. So the gate
reconstructs A.4's **outcome** (a directory of valid reports the scanner admits none of,
for one attributable reason) by a different route, and says so in the file. Re-breaking the
parser to get a more literal replay would test the parser, not the log.

### What this does NOT do

It records the application's behaviour, not the money. `crates/audit/` already covers the
money side durably (double-entry SQLite, story 3-21's crash proof). The two schemas are
deliberately not entangled.

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

- ADR: [`0096-the-cockpit-records-what-it-did.md`](../planning-artifacts/architecture/decisions/0096-the-cockpit-records-what-it-did.md)
- Runbook: [`docs/runbooks/operator-session-log.md`](../../docs/runbooks/operator-session-log.md)
- Trace: `REQ-OPERATOR-SESSION-LOG-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 6 (Remediation, Infra & Governance)
