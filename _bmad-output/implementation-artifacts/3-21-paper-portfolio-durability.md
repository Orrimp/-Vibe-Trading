# Story 3.21: paper-portfolio-durability

Status: backlog

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

## Tasks / Subtasks

- [ ] Inventory what is persisted today vs what the survival contract needs to claim (start from the durable live-equity-history work and the audit ledger).
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
