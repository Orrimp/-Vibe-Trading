# Story 3.21: paper-portfolio-durability

Status: ready-for-dev

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
- [ ] Dev: version + migrate-or-fail-loud; crash-consistency test; restored-from surfacing.
- [ ] Runbook/in-app documentation of the contract.

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

- Trace: `REQ-PAPER-PORTFOLIO-DURABILITY-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP)
