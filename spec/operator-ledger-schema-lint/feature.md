---
slug: operator-ledger-schema-lint
version: 0.1.0
status: draft
owner: analyst
priority: P2
updated: 2026-05-29
---

# Operator-ledger schema-lint — v0.1.0

> **Pick C Wave 1 promoted feature (orchestrator hygiene compounder
> trio).** Per
> [`spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`](../dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md)
> this is one of three trio pillars (~0.5 dev days), biased toward
> DURABLE: a `scripts/operator_ledger_check.py` script that enforces
> the schema of
> [`spec/dev-notes/operator-side-pending-ledger.md`](../dev-notes/operator-side-pending-ledger.md)
> and auto-detects stale FAILED rows for operator escalation.

## Why

Per the
[`weekly-retro-2026-05-27-to-2026-05-29 § What to fix / improve #5`](../dev-notes/weekly-retro-2026-05-27-to-2026-05-29.md#what-to-fix--improve)
finding: operator visual-verify recipe backlog accumulates across
session boundaries — Bug #64 attempt-2 visual-verify; Yahoo v0.1.4
bulk fetch (now done); cockpit-toast-queue smoke tests; etc. The
**ledger itself** (`operator-side-pending-ledger.md`) was created
2026-05-29 per the retro fix-improve #5 proposed-fix: "orchestrator
maintains a single ledger that survives session boundaries; presenter
decks link in rather than each authoring their own."

The ledger has frontmatter conventions (status enum, append-only) but
NO SCHEMA ENFORCEMENT. This brief **upgrades** the ledger to a
schema-enforced living document via `scripts/operator_ledger_check.py`,
which asserts:

1. The table schema (date / recipe / cost / unblocks / status / notes
   columns).
2. The status enum (`{pending, FAILED, done, cancelled}`).
3. **Stale FAILED escalation**: any row with `status: FAILED` older
   than 7 days surfaces with an escalation reminder ("this FAILED
   recipe has been in the ledger ≥ 7 days without resolution — route
   to analyst for re-investigation OR mark as cancelled").
4. Done rows have a completion date (in the dedicated "Completed"
   column or via Q-LED-NOTE shape ratification).

Per the
[`process-tooling-survey-2026-05-29.md § Pick C`](../dev-notes/process-tooling-survey-2026-05-29.md#pick-c--orchestrator-hygiene-queue-staleness-script-a--adr-registry-atomic-write-contract-b--pending-verifications-ledger-c):
"Pending-verifications ledger consolidates a chronic carry-over class
(Bug #64 visual-verify, Yahoo bulk fetch, toast-queue smoke tests)."
Pick C upgrades this to a **schema + lint script** that enforces the
row format AND auto-detects stale FAILED rows for analyst escalation.

Three layered consequences:

- **Codified-contract preservation.** The ledger frontmatter
  conventions are only as good as their enforcement; the lint
  converts "convention" to "structurally-enforced schema."
- **Stale FAILED escalation**. Without the lint, FAILED rows
  accumulate silently — Bug #64 D.1.1 is a perfect example (FAILED
  2026-05-29 with an analyst investigation linked but no closure
  trigger). The 7-day stale-FAILED escalation reminds the
  orchestrator to surface the carry-over at the next session.
- **Schema stability for future ledger evolution.** As the ledger
  shape evolves (new columns, new status values), the lint becomes
  the change-management surface — any operator-recipe shape
  evolution requires a lint schema update, which is a structural
  signal.

Per process-tooling-survey: **MEDIUM per-cycle benefit, SMALL
investment (~0.5d), LOW maintenance**. Operationalises retro fix
proposal #5 at the schema layer; complements the orchestrator's
already-existing ledger-maintenance contract.

## Requirements

### R1 — Script invocation + scope

- **R1.1** A new script `scripts/operator_ledger_check.py` exists,
  invokable as `python3 scripts/operator_ledger_check.py` from repo
  root. Sibling pattern to `scripts/spec_brief.py`,
  `scripts/spec_lint.py`, `scripts/hash_report.py` (Python 3 stdlib;
  no requirements.txt).
- **R1.2** Script reads
  `spec/dev-notes/operator-side-pending-ledger.md` and parses the
  three tables:
  - `## Pending recipes` (active rows)
  - `## Done recipes (audit trail)`
  - `## Cancelled recipes (audit trail)`
- **R1.3** Per-table schema:
  - **Pending recipes**: columns are `Date surfaced | Recipe | Cost
    | Unblocks | Status | Notes`. Status enum:
    `{pending, FAILED, done, cancelled}`. Date format ISO `YYYY-MM-DD`
    (with optional bold/markdown formatting like `**FAILED 2026-05-29**`
    in status cell per existing Bug #64 D.1.1 row).
  - **Done recipes**: columns are `Date surfaced | Recipe | Cost |
    Completed | Outcome`. The `Completed` column holds the completion
    date.
  - **Cancelled recipes**: columns are `Date surfaced | Recipe | Cost
    | Cancelled | Reason`.
- **R1.4** Script asserts:
  - (a) Per-row column count matches the per-table schema.
  - (b) Status cell value normalizes (strip bold/italic/links) to a
    valid enum value.
  - (c) `Date surfaced` parses as ISO date.
  - (d) Done rows have a completion date in the `Completed` column.
  - (e) FAILED rows in `## Pending recipes` older than 7 days
    (relative to today's date) surface an **escalation reminder**.
  - (f) Per Q-LED-NOTE ratification: if FAILED rows are required to
    have a follow-up dev-note citation in the `Notes` column,
    assert the citation matches a `spec/dev-notes/*.md` path.
- **R1.5** Script exits with code 0 on clean; code 1 on schema
  violation OR stale FAILED escalation; code ≥ 2 on script failure.

### R2 — Operator-friendly diff output

- **R2.1** On schema violation or stale FAILED, script writes to
  stderr a markdown-formatted diff message per bundle Q-HYG-EMIT
  ratification:
  ```text
  operator-ledger-check: <N> issue(s) detected
  | issue | row | observed | expected | action |
  |-------|-----|----------|----------|--------|
  | stale-failed | 2026-05-28 Bug #64 D.1.1 visual-verify | FAILED 2026-05-29 (1 day old) | escalate after 7 days | (no action — within window) |
  | schema-status-enum | 2026-05-XX Foo Bar | status: "blocked" | one of {pending, FAILED, done, cancelled} | normalize cell value |
  | missing-completion-date | 2026-05-27 ETH-USD populate | Completed cell empty | ISO date | fill Completed cell |
  ```
- **R2.2** Each issue row identifies the issue class, the offending
  row (date + recipe excerpt), the observed state, the expected
  state, and the suggested action.
- **R2.3** Clean run produces ZERO output (silent success).
- **R2.4** Failure (exit ≥ 2) produces an error-prefixed message
  with file:line context.
- **R2.5** Stale FAILED rows that ARE within the 7-day window emit
  a "soft warning" line at info level (not stderr-stderr; doesn't
  fail the lint) — per Q-LED-WHEN ratification, the orchestrator
  uses this for "carry-over status" reporting at session start.

### R3 — Wire-up

- **R3.1** Per Q-LED-WHEN ratification, the script wires up as
  either:
  - (a) Orchestrator pre-flight at session start (Recommended
    DURABLE) — invoked alongside the Queue-staleness sweep per the
    bundle's shared pre-flight pattern. Catches stale FAILED rows
    at the natural carry-over point.
  - (b) Pre-commit hook on `spec/dev-notes/operator-side-pending-ledger.md`
    modification only — catches schema violations at commit time
    but not stale-FAILED escalation.
  - (c) Both (a) + (b) — belt-and-suspenders; v0.2.0+ scope.
- **R3.2** v0.1.0 ships with Option A (Recommended DURABLE);
  orchestrator includes the script's drift output in the next
  session header.
- **R3.3** The ledger frontmatter at
  `spec/dev-notes/operator-side-pending-ledger.md` gets amended in
  this feature's M-T1 close with a 1-line "validated by
  `scripts/operator_ledger_check.py`" note. This brief OWNS the
  ledger frontmatter amendment per bundle K4 ownership-table; sibling
  briefs do not touch the ledger.
- **R3.4** AGENT.md cross-reference: the bundle direction declares
  the ledger contract codification at AGENT.md. v0.1.0 adds a 1-line
  cross-reference to the ledger from AGENT.md § (TBD section — most
  likely under the Queue pre-flight section or a new "Pending
  operator verifications" subsection). Architect M-T1 picks the
  exact section.

### R4 — Self-test (script smoke)

- **R4.1** Script has a `#[cfg(test)]`-equivalent self-test in
  `scripts/tests/test_operator_ledger_check.py` (or inline
  `--self-test` flag per architect M-T1):
  - Asserts clean ledger → exit 0
  - Asserts schema-status-enum violation → exit 1 with the right
    issue class
  - Asserts stale-FAILED escalation fires at 8 days old
  - Asserts stale-FAILED does NOT fire at 1 day old (within window)
  - Asserts missing-completion-date in Done table → exit 1
  - Asserts exclude-rule on the Cancelled table (no completion
    date required)
- **R4.2** Self-test runs in < 1 s.

### R-NR — Non-regression contract

- **R-NR.1** Script is READ-ONLY on
  `spec/dev-notes/operator-side-pending-ledger.md` — never
  mutates the ledger. (Orchestrator continues to maintain the
  ledger per the existing contract.)
- **R-NR.2** Zero new Cargo.toml deps.
- **R-NR.3** Zero new external Python deps.
- **R-NR.4** `bash scripts/verify_anchors.sh` → all-PASS byte-
  identical pre/post.
- **R-NR.5** Ledger frontmatter amendment (R3.3) is the only edit
  to `operator-side-pending-ledger.md` from this brief; all rows
  byte-identical.
- **R-NR.6** Append-only contract preserved: lint never overwrites,
  never auto-fixes, never removes rows.
- **R-NR.7** No new clippy / fmt deltas (Python script).

## Falsifiers (K)

- **K1 — Markdown table parsing fragility on the existing ledger
  row.** Per the bundle direction § Risk K3: the existing Bug #64
  D.1.1 row has a multi-line `Notes` cell with embedded links + bold
  + nested timeline. Python markdown parsing of pipe-separated cells
  must handle escaped pipes (`\|`) AND embedded links with brackets
  that don't close on cell boundaries. **Mitigation**: feature.md
  R1.3 specifies markdown-tolerant cell parse (strip
  bold/italic/links before enum match); architect M-T1 ratifies the
  parse shape at D-LED-2. The existing 5 rows in the ledger
  (1 pending + 4 done at v0.1.0 author time) become the regression
  test suite at R4.1.
- **K2 — Schema drift as operator-recipe shape evolves.** The
  v0.1.0 schema (date / recipe / cost / unblocks / status / notes
  for Pending; date / recipe / cost / completed / outcome for Done)
  may evolve as new recipe shapes surface. **Mitigation**: the
  schema is a NAMED CONSTANT in the script (architect M-T1 picks the
  module-top `SCHEMA` dict); any operator-recipe shape change
  requires a v0.1.x lint update — which is a STRUCTURAL change-
  management signal that's a feature, not a bug. Operator can
  add new optional columns without breaking the lint (lint accepts
  ≥ N columns, asserts presence of required ones).
- **K3 — Stale-FAILED 7-day threshold over- or under-tuned.** 7
  days may be too tight (operator may have legitimate 2-week
  visual-verify windows when traveling) or too loose (FAILED rows
  may demand same-day escalation if blocking a feature). **Mitigation**:
  feature.md K3 documents the 7-day default as a tunable named
  constant `STALE_FAILED_DAYS = 7`. v0.1.0 ships at 7 days; operator
  feedback during the first 2 weeks tunes via v0.1.x. The escalation
  is a SOFT warning (lint exits 1 but operator can override) — not
  a HARD block on legitimate operator work.
- **K4 — Append-only contract violated.** If a future agent
  modifies an existing ledger row in-place (e.g. updates status
  from `pending` to `FAILED` by editing the status cell directly),
  the git history would lose the append trail. **Mitigation**: the
  ledger's append-only contract is a SOFT contract enforced by the
  orchestrator's habit, not the lint. v0.1.0 lint does NOT
  enforce append-only (that would require git-diff awareness on the
  ledger file — out of scope). Documented as a v0.2.0+ candidate
  if the soft contract proves insufficient.

## Hypotheses (H)

- **H1 — Script wall-clock ≤ 0.2 s on current ledger.** The
  ledger is a single markdown file with ≤ 20 rows at any point.
  Parse + status enum check + date age computation completes in
  sub-second.
- **H2 — Script ≤ 200 LoC.** Markdown table parse + per-row schema
  check + stale-date computation + emit logic fits within ~150 LoC;
  +50 LoC self-test. Matches the analyst's ~0.5d estimate.
- **H3 — Zero existing tests break.** Pure script addition.
- **H4 — Script catches Bug #64 D.1.1 FAILED escalation at day
  7+.** The Bug #64 D.1.1 row is FAILED 2026-05-29; at next session
  (assumed within 1 day), within-window soft-warning. At 2026-06-05
  (7 days), escalation fires. Confirmed by self-test R4.1.

## Operator decisions

### Q-LED-WHEN — Lint wire-up timing

**Q.** When does the lint run — orchestrator session pre-flight
(alongside queue-staleness), pre-commit on ledger modification, OR
both?

**(Recommended — DURABLE) Option A — orchestrator session pre-flight
only at v0.1.0.** Catches the stale-FAILED escalation at the natural
carry-over point (session start). Schema violations are rare (only
fire when the ledger format drifts, which is itself a structural
signal). Sub-second wall-clock; rolls up under the bundle's shared
pre-flight pattern.

**Cost.** ~0 — the script integrates with the orchestrator's existing
session-start sweep (defined by sibling
`queue-staleness-reconciliation`).

**Rationale (DURABLE).** Per AGENT.md 2026-05-28: stale-FAILED
escalation is a SESSION-START surface, not a commit-time surface;
schema violations are rare AND visible at commit-time human review.
Pre-commit hook adds complexity without much value (the lint would
fire on the orchestrator's own ledger updates, requiring suppression).
Both-modes (Option C) is v0.2.0+ scope.

**Option B (cheap fallback — REJECTED at analyst level).** Pre-commit
hook on ledger modification only. Catches schema drift at commit
time but MISSES the stale-FAILED escalation (no commit means no
trigger). **Rejected** — the primary value class is stale-FAILED
escalation; missing that defeats the purpose.

**Option C.** Both pre-commit + session pre-flight. v0.2.0+ scope;
v0.1.0 ships Option A only.

**Default**: A (Recommended DURABLE).

### Q-LED-NOTE — FAILED rows require follow-up dev-note citation

**Q.** Does the v0.1.0 lint enforce that every FAILED row carries a
follow-up dev-note citation in the `Notes` column (e.g. the existing
Bug #64 D.1.1 row links to
`bug-64-d11-attempt-3-investigation-2026-05-29.md`)?

**(Recommended — DURABLE) Option A — YES, FAILED rows MUST cite a
follow-up dev-note in Notes.** Structurally enforces the
"investigation-on-failure" pattern the orchestrator has been doing
informally. The Bug #64 D.1.1 row sets the precedent — every FAILED
recipe gets an analyst investigation dev-note, which the operator can
navigate to from the ledger. Without enforcement, FAILED rows may
accumulate with vague "TODO investigate" notes that lose context
across sessions.

**Cost.** ~0 — the lint check is one regex (`spec/dev-notes/.*\.md`
path match in the Notes cell).

**Rationale (DURABLE).** Per AGENT.md 2026-05-28: durable operator
workflow ties EVERY FAILED recipe to a documented investigation. The
cheap path (allow vague Notes) lets context decay across session
boundaries — exactly the regression class the ledger exists to
prevent. Codify the link.

**Option B (cheap fallback — REJECTED at analyst level).** Permissive —
FAILED rows MAY have a dev-note citation but it's optional. **Rejected**
per the regression-decay argument: without enforcement, the
investigation-on-failure habit degrades over sessions; this is the
exact pattern the bundle is designed to prevent.

**Default**: A (Recommended DURABLE).

## Verdict tree (pre-drawn)

| Q-LED-WHEN \ Q-LED-NOTE | Q-LED-NOTE=(a) require dev-note | Q-LED-NOTE=(b) permissive |
|---|---|---|
| **Q-LED-WHEN=(a) session pre-flight** | **DURABLE — Recommended.** Session-start escalation + structurally-enforced investigation links; bundle-aligned. | INCONSISTENT — escalation fires but investigation context decays; defeats half the purpose. |
| **Q-LED-WHEN=(b) pre-commit only** | INCONSISTENT — investigation links enforced but escalation misses (no commit trigger). | REJECTED — misses both primary value classes. |

## Design

_architect M-T1 fills this_

## Implementation

_developer M-DEV fills this_

## Verification

_tester M-FINAL links report here_

## Changelog

- 2026-05-29 (analyst): Feature brief authored under Pick C Wave 1
  promotion per
  [`spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`](../dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md).
  R1-R4 + R-NR (7 clauses) + K1-K4 + H1-H4 + Q-LED-WHEN +
  Q-LED-NOTE + pre-drawn 4-cell verdict tree. Both Qs bias DURABLE
  per AGENT.md 2026-05-28. Ledger frontmatter amendment (R3.3) +
  AGENT.md cross-reference (R3.4) OWNED by this brief per bundle K4
  ownership-table. Trace row `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001`
  opened `proposed`. HANDOFF → architect (M-T1 + bundle parallel
  block with `queue-staleness-reconciliation` +
  `adr-registry-atomic-lint` siblings).
