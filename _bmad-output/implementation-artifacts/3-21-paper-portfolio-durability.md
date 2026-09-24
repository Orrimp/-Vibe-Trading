# Story 3.21: paper-portfolio-durability

Status: done

<!-- Created 2026-08-04 by the adversarial product review
     (docs/dev-notes/product-review-2026-08-04.md, finding 12). -->

## Story

As the operator paper-trading a simulated €200 forward,
I want my paper portfolio to survive a crash, a restart, and a release,
so that the one artifact I am actually invested in has at least the durability ceremony
that anchors, evidence rows, and trace entries already get.

## Acceptance Criteria

1. **Documented survival contract.** A single place (runbook or in-app help) states exactly
   what survives a process kill: positions, cash, fills, the equity history, the plan that
   produced them — each named, each with its on-disk location. Today this is inferable from
   code and from nowhere else.
2. **Crash-consistency proof.** A test kills the writer mid-session (or simulates a torn
   write) and proves the paper state reloads to a coherent point — no half-applied fill, no
   position without its cash leg. The double-entry ledger's exact-cent reconciliation is the
   natural invariant to assert on reload (AD-9).
3. **Schema-migration story.** Persisted paper state carries a version; loading an older
   version either migrates or fails loudly with an actionable message. Silent field-drop on
   deserialization is not acceptable for the user's own money-shaped data.
4. **The user can see it.** The Live view surfaces "restored from <timestamp>" (or "fresh
   session") so a restart is legible rather than mysterious.
5. Standing floor: anchors 119/119; spec-lint PASS; Decimal money throughout (AD-9); no
   change to strategy/gate behaviour; UI additions verified at the render layer per AD-10.

## Inventory result (task 1, done 2026-08-04 by the orchestrator — operator chose "inventory first")

**The story does NOT shrink to document-only, and the money side is stronger than
the review feared. Concrete findings:**

- **Money/equity durability is structurally sound.** The ledger is SQLite with
  *balanced double-entry transactions written atomically per fill*, debits == credits
  enforced per transaction (`crates/audit/src/journal.rs`); the live equity series is
  durable through `LiveEquityStore` over that same ledger (ADR-0052,
  `crates/audit/src/equity_store.rs`), `Money<Usdt>`/Decimal throughout. Crash
  atomicity is provided by the storage layer — AC2 reduces from "build
  crash-consistency" to **"prove it with a test"** (kill mid-session, reload, assert
  the ledger reconciles exactly and no position lacks its cash leg).
- **REAL DEFECT on the state side (AC3):** `lab::persistence::decode` (`:196-217`)
  handles BOTH a version mismatch and any parse error by logging a `warn!` and
  **silently returning cold-start defaults**. The user's saved session (strategy,
  pair, range, compare set) is discarded with no actionable message and no UI signal
  — a `tracing` line they will never see. This is exactly the "silent field-drop is
  not acceptable for the user's own data" concern, and it is live today, not
  hypothetical.
- **AC1 gap confirmed:** the survival facts exist but are spread across ADR-0052, the
  journal module docs, and `persistence.rs`'s schema comment. There is no single place
  a user or a future maintainer can read what survives a kill.
- **AC4 is now more important, not less:** with a silent reset in the load path, the
  user cannot distinguish "restored" from "reset to defaults" — the surfacing is the
  only thing that would make the defect above visible when it fires.

**Revised scope:** AC1 (write the contract), AC2 (prove the existing atomicity — do
not rebuild it), AC3 (fail loudly + surface, rather than silently cold-start), AC4
(restored-from / reset-to-defaults surfacing). No persistence rewrite.

## Tasks / Subtasks

- [x] Inventory what is persisted today vs what the survival contract needs to claim — **done 2026-08-04, result above**.
- [x] Dev: version + migrate-or-fail-loud; restored-from surfacing — **done 2026-09-24**.
- [x] Crash-consistency test (AC2) — **done 2026-09-24**, `crates/audit/tests/`.
- [x] Runbook documentation of the contract — **done 2026-09-24**,
      [`docs/runbooks/paper-state-durability.md`](../../docs/runbooks/paper-state-durability.md).

## Dev record (2026-09-24)

**The inventory understated the defect, and the correction is the story's main
finding.** It named `lab::persistence::decode` as silently cold-starting. Checking the
blast radius before fixing it showed that nothing called the feature at all:
`Cockpit::boot` and the whole `PersistenceDebouncer` were reachable only from their own
unit tests (`scripts/callers.sh`, CodeGraph ∪ grep), and both shipped binaries
constructed the cockpit with `Cockpit::new()`. **The Lab session was written to disk
never and read from disk never**, from `c654f31` (2026-05-17) until this change.
Recorded as **bug-log #102**.

So the work landed in that order — wire it, then fix the load path — because a
migrate-or-fail-loud contract on a load that never runs is ceremony.

| AC | What shipped |
|---|---|
| AC1 | `docs/runbooks/paper-state-durability.md` — the table of what survives, where, and written when; the money side and its one unstated limit (no explicit SQLite `journal_mode`); the session side and its one measured gap. |
| AC2 | the crash-consistency test in `crates/audit/tests/`, with a negative control. |
| AC3 | `decode` returns `Result<LabState, FailureReason>` instead of swallowing; `restore` returns the state AND a `RestoreOutcome`. A file that cannot be used is **renamed aside before returning**, because the debounced writer would otherwise overwrite the operator's saved session with cold-start defaults ~500 ms later — the failure mode was data LOSS, not data ignored. No migration exists (v1 is the only version), so every mismatch fails loudly and names both versions. |
| AC4 | the Lab toolbar renders "Fresh session" / "Restored from &lt;ts&gt;" / the failure and the kept-aside path in `DOWN_500`. Proven at the pixels by `crates/ui/tests/lab_session_restore_render.rs` — three gates plus the negative control that two identical outcomes render byte-identically. |
| AC5 | anchors 119/119; spec-lint PASS; no `f64` added; no strategy/gate behaviour touched; UI proven at the render layer. |

**Two deviations, both deliberate, neither silent:**

1. **AC4 says "the Live view"; the notice is on the Lab screen.** AC4 was written in
   August, before the inventory knew which state was actually at risk. The state that
   restores from a file is the Lab session, so the notice belongs where that state is.
   The Live view's paper portfolio restores from the ledger, which is a different
   mechanism with a different failure mode. **Operator call if you want a second
   surfacing on Live for the equity hydrate** — it is not built here.
2. **No forced flush at window close.** `iced::application(..).run()` consumes the
   application state, and the `app_state` still in scope afterwards is a pre-boot clone,
   so there is no correct place to force a final write. A selection changed in the last
   ~500 ms before the window closes is lost. Stated in the code at the call site and in
   the runbook rather than papered over.

## Dev Notes

- Origin: product review 2026-08-04 finding 12 — every durable artifact in this repo has a
  formal immutability/verification contract (anchors byte-frozen and gated, trace rows
  lint-enforced, evidence reports SHA-locked) EXCEPT the user's own paper portfolio, which
  has none.
- Related shipped work to build on rather than duplicate: durable live-equity history
  (story 2-16), the double-entry audit ledger with exact-cent reconciliation, and the
  operator ledger schema lint (story 6-5).
- Do-not-build register: not implicated (durability of existing state; no new surface, no
  live trading — this is explicitly PAPER state).
- Scope guard: this is NOT a persistence rewrite. If the inventory in task 1 shows the
  contract is already met, the story reduces to AC1 + AC4 (document it, surface it) and
  should say so honestly rather than manufacture work.

### References

- ADR: [`0094-the-lab-session-persists-and-says-so.md`](../planning-artifacts/architecture/decisions/0094-the-lab-session-persists-and-says-so.md)
- Bug log: `#102` (the unwired persistence path), FIXED here
- Runbook: [`docs/runbooks/paper-state-durability.md`](../../docs/runbooks/paper-state-durability.md)
- Trace: `REQ-PAPER-PORTFOLIO-DURABILITY-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP)
