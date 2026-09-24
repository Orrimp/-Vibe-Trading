---
adr: 0096
title: The cockpit records what it did, with a denominator
status: accepted
date: 2026-09-24
supersedes: none
superseded-by: none
---

# ADR-0096: The cockpit records what it did, with a denominator

## Context

Five defects in this repo's bug log shipped past a green suite (`#65`–`#69`). The common
thread is not that the tests were weak in some general way — it is that **nothing
independent of the tests recorded what the software actually did**.

`#66` A.4 is the canonical case. `report::sma` emitted its `strategy:` frontmatter keys
unindented, `compare::cache::parse_frontmatter` filed them top-level, and
`scan_one_root`'s `let Some(..) = fm.get("strategy.id") else { continue }` therefore threw
away **every engine-written report**. The Compare screen rendered perfectly and showed an
empty matrix.

An empty Compare matrix is also what a fresh checkout shows. Nothing in the product could
tell those two states apart, and the defect survived weeks until a code review found it by
reading.

## Decision

**D1 — The unit is an operator-visible action and its outcome, not a tracing line.**
Six events: `session_started`, `screen_opened`, `run_started` (with the inputs that define
the run), `run_finished` (result or error), `scanned`, `artifact_written`. A tracing
firehose is what this repo already had; it did not help, because it records what the code
said rather than what the operator did.

**D2 — `scanned` carries a DENOMINATOR, and that is the whole point.**
`ScanTally { discovered, admitted, rejected: BTreeMap<Rejected, usize> }`. `0 admitted of
0 discovered` says "there was nothing here". `0 admitted of 40 discovered, all
`missing_strategy_id`" says "there was plenty here and I used none of it". Those are
different sentences and until this ADR the product could only say the first.

**D3 — The rejection vocabulary is a CLOSED enum.** `Rejected` names seven reasons. An
open `String` would let a future skip-path be added without anyone deciding it deserves a
name, and an unnamed skip path is exactly how `#66` A.4 stayed invisible: `scan_one_root`
had seven bare `continue`s and no vocabulary for distinguishing them. Naming them is the
substance of this change; the log file is the delivery mechanism.

**D4 — The scanner stays pure; the caller logs.** `scan_report_roots` returns
`(cache, ScanTally)`. It does not know the log exists. This keeps the denominator provable
without installing a sink — the acceptance test asserts on the returned tally — and keeps a
UI concern out of a filesystem scan.

**D5 — Local, plain, and inspectable.** One append-only JSONL file per session under
`$XDG_STATE_HOME/trading/sessions/`, readable with `cat`. **No network call of any kind**:
no telemetry service, no analytics, no crash reporting, no identifier. The directory lives
outside the repository, so there is no `.gitignore` rule to forget. A `README.md` is
written beside the logs on first use, because the person most likely to wonder what those
files are is the person looking at them.

**D6 — Off is structural, not a branch.** The sink is `Option<Arc<dyn SessionSink>>` on the
`Cockpit` — the same switch shape as `lab_state_path` (ADR-0094 D3). Every fixture, gallery
and test leaves it `None`, so none of them can write to the operator's real log, and only
`Cockpit::boot` arms it. `TRADING_SESSION_LOG=0` disarms the binaries. A file that cannot be
opened costs one `warn!` and nothing else: a cockpit that refused to start because it could
not open its own diary would be a worse bug than the one this catches.

**D7 — The acceptance test is a DISCRIMINATION test.**
`session_log_replays_66_a4.rs` builds a corpus of valid reports the scanner admits none of
and asserts its record **differs** from an empty corpus's, with a positive control proving
the corpus differs from the healthy one by exactly the strategy block. A presence test —
"the log wrote something" — would pass on a log that says "0 cells" for both, which is
precisely the log that would not have caught `#66` A.4.

**D8 — Retention is bounded at 30 sessions and stated where it applies.** A few weeks of
ordinary use at a few KB each. It is not a privacy control; nothing leaves the machine.

## Alternatives considered

- **Structured `tracing` spans with a file subscriber** — rejected. It records what the
  code chose to say, at the granularity the code chose, and this repo already had that
  while `#66` A.4 was live. The `continue`s that dropped forty reports emitted nothing,
  and a subscriber cannot invent a denominator that the scanner never computed.
- **An open `String` reason on rejections** — rejected under D3. It would make the next
  silent skip-path cheap to add, which is the failure mode.
- **Log from inside `scan_one_root`** — rejected under D4: it puts a UI sink behind a
  filesystem walk and makes the denominator unprovable without installing one.
- **On by default with no off-switch** — rejected: AC4 asks for zero cost when off, and
  the `Option` shape gives it structurally rather than by a runtime check.

## Consequences

- `scan_report_roots` returns a tuple; five call sites updated, `scan_spec_tree` discards
  the tally by name.
- The Compare cold-boot scan is the first place in this product that can state emptiness
  with a denominator. Every event in the vocabulary has a producer: `screen_opened` on
  `SwitchScreen`, the Lab `run_started`/`run_finished` pair, `scanned` on the Compare
  cold-boot index, and `artifact_written` on a saved forward-plan export (its failure arm
  logs a `run_finished` with the reason). **No variant ships declared-and-never-raised** —
  that is the shape bug-log `#102` and `#95` punish, and an unused event in a vocabulary
  invented to name things would have been a poor joke.
- **Moral**: "is it tested?" and "can the program say what it did?" are different
  questions. A green suite answers the first; five defects in this log needed the second.
