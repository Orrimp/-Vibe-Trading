---
adr: 0094
title: The Lab session persists, and the cockpit says whether it came back
status: accepted
date: 2026-09-24
supersedes: none
superseded-by: none
---

# ADR-0094: The Lab session persists, and the cockpit says whether it came back

## Context

Story 3-21's inventory (2026-08-04) named one defect: `lab::persistence::decode`
handles both a schema-version mismatch and any parse error by logging a `warn!` and
returning cold-start defaults, so the operator's saved selection is discarded behind a
line they will never see.

Checking the blast radius before fixing it found a larger one. Nothing called the
feature at all:

```
crates/ui/src/state.rs:1529   Cockpit::boot            <- callers: its own unit tests
crates/ui/src/lab/persistence.rs:307  PersistenceDebouncer
                              :329  flush_if_due       <- callers: its own unit tests
                              :341  force_flush        <- callers: its own unit tests
```

Both shipped binaries built the cockpit with `Cockpit::new()`. The Lab session was
written to disk **never** and read from disk **never**, from `c654f31` (2026-05-17,
ui-rethink-phase-a-lab Wave 2, T-D-17) until this decision. Nine unit tests passed
throughout, because each one calls the API directly — a test that constructs the thing
it tests cannot observe that the product does not construct it. Bug `#54` (`799543a`)
later changed the cold-start default on the belief that the cold-start path was
user-visible; it was, because *every* start was a cold start. Recorded as bug-log
**#102**; the same shape as **#95** (declared at nine sites, read at one) and **#81**
(an arm whose loader never compiled).

## Decision

**D1 — Wire it before fixing it.** `Cockpit::boot` replaces `Cockpit::new` in
`bin/cockpit_live.rs` and in the non-fixtures branch of `bin/cockpit.rs`. A
migrate-or-fail-loud contract on a load that never runs is ceremony.

**D2 — The writer is driven from `ui::state::update`, not from either binary.**
`update` is the one seam both cockpits share; a per-binary subscription would have to
be added twice and could be forgotten once, which is how this got here. `persists_lab_state(&msg)`
marks dirty on the five messages that change something the schema records — an explicit
list, not a `Message::Lab*` prefix match, because run progress and run completion are
transient and would rewrite the file on every progress tick. `update` is split into a
thin wrapper plus `update_state_machine` so the early `return`s in two arms cannot skip
the flush.

**D3 — `lab_state_path: Option<PathBuf>` is the single switch that arms the writer.**
`None` means persistence is off, and every fixture, gallery and test constructor sets it
so. Only `boot` sets `Some`. A demo cockpit therefore cannot write over the operator's
real session file, and that property is visible at each constructor rather than inferred.

**D4 — A file the cockpit cannot use is renamed aside BEFORE the load returns.**
`<name>.unreadable-<unix-seconds>`, best-effort. This is the whole point of the
mechanism: the debounced writer overwrites the path unconditionally ~500 ms after the
next interaction, so without the rescue a load failure would **destroy** the saved
session rather than ignore it. When the rename itself fails the outcome says so, rather
than promising a rescue that did not happen.

**D5 — The outcome is a value, not a log line.** `decode` returns
`Result<LabState, FailureReason>`; `restore` returns `(LabState, RestoreOutcome)` —
`Fresh` / `Restored { saved_at }` / `Failed { reason, preserved_at }`. There is no
migration: version 1 is the only version there has ever been, so every mismatch fails
loudly and names both versions. A future v2 migrates in `decode`, and only then does
that arm narrow.

**D6 — The Lab toolbar renders the outcome, and a render test proves it.**
"Fresh session" / "Restored from &lt;ts&gt;" / the failure and the kept-aside path in
`DOWN_500`. `crates/ui/tests/lab_session_restore_render.rs` asserts the three states
paint differently in a measured band (y 0..60 at 1600×900), that only the failure paints
`DOWN_500` there, and that nothing below the band moves, and that an UNARMED cockpit
paints none of it — with the negative control FIRST: two renders of the SAME outcome must be byte-identical, or every "the frames
differ" assertion after it is satisfied by renderer jitter. That control exists because
bug-log #96's gate passed on a scrollbar thumb.

**D7 — Two limits are stated rather than papered over.**

- **The notice is on the Lab screen, not the Live view.** AC4 said "Live view"; it was
  written before the inventory knew which state was at risk. The state that restores
  from a file is the Lab session. The Live view's paper portfolio restores from the
  ledger — a different mechanism with a different failure mode. A second surfacing for
  the equity hydrate is an open operator call, not built here.
- **There is no flush at window close.** `iced::application(..).run()` consumes the
  application state, and the `app_state` still in scope afterwards is a pre-boot clone,
  so there is no correct place to force a final write. A selection changed in the last
  ~500 ms before the window closes is lost. Stated at the call site and in the runbook.

**D8 — `restore_or_default` is deleted.** After D1 its only callers were its own tests
— the exact condition this ADR exists to correct. Its two tests now exercise `restore`.

## Alternatives considered

- **Fix `decode` as the inventory described and stop** — rejected on the evidence. It
  would have made a fail-loud contract correct inside a module the product never reaches,
  and the story would have shipped claiming durability the operator does not have.
- **Force a flush at shutdown by putting the `Cockpit` behind an `Arc<Mutex<..>>`** —
  rejected. It buys a ~500 ms window at the cost of a lock on the UI state, for a session
  selection whose loss is recoverable in one click. The money side is the ledger, which is
  atomic per fill and proven so by `crates/audit/tests/crash_consistency.rs`.
- **Drive the writer from a dedicated 1 Hz subscription** — rejected as redundant: the
  existing server-time tick already guarantees an `update` at least once a second, so the
  debounce is honoured without a second recipe to keep in sync.
- **Keep the unreadable file in place and just report it** — rejected: the reporting
  would be true for about 500 ms, after which the writer would have overwritten the thing
  the message tells the operator to go and look at.

## Consequences

- The Lab selection now survives a restart. `~/.config/trading/cockpit-lab-state.json`
  is created by the shipped cockpit for the first time.
- **No visual baseline moves — but only because of D6's gate.** The first check for this
  ("no byte-exact baseline renders the Lab screen") was wrong: `render_snapshots`'
  `chart_screen` fixtures ARE the Lab screen under an older name, and the ungated notice
  painted "Fresh session" into 8 of them. The 56-baseline corpus is unchanged in this
  commit; nothing was re-captured, and no operator baseline approval was needed.
- The survival contract for *all* paper state, money side included, is written down once
  in [`docs/runbooks/paper-state-durability.md`](../../../../docs/runbooks/paper-state-durability.md),
  with two facts the inventory had not recorded: the ledger runs SQLite in `delete`
  (rollback-journal) mode, so `ledger.db` alone is not a consistent snapshot while the
  cockpit runs; and `journal::verify_balance` is tolerance-based at 1e-8, so AD-9's
  "exact-cent reconciliation" is an epsilon in code rather than an equality.
- **Moral, recorded in bug-log #102**: "is it tested?" and "is it reachable from `main`?"
  are different questions, and a green suite answers only the first. Ask
  `scripts/callers.sh` of any feature you are about to extend — the extension inherits its
  reachability, not its test count.
