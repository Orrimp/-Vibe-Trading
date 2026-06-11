---
slug: operator-ledger-schema-lint
version: 0.1.0
status: shipped
owner: shipped
priority: P2
updated: 2026-05-30
---

# Operator-ledger schema-lint — v0.1.0

> **Pick C Wave 1 promoted feature (orchestrator hygiene compounder
> trio).** Per
> [`spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`](../dev-notes/archive/2026-Q2/pick-c-orchestrator-hygiene-2026-05-29.md)
> this is one of three trio pillars (~0.5 dev days), biased toward
> DURABLE: a `scripts/operator_ledger_check.py` script that enforces
> the schema of
> [`spec/dev-notes/operator-side-pending-ledger.md`](../dev-notes/operator-side-pending-ledger.md)
> and auto-detects stale FAILED rows for operator escalation.

## Why

Per the
[`weekly-retro-2026-05-27-to-2026-05-29 § What to fix / improve #5`](../dev-notes/archive/2026-Q2/weekly-retro-2026-05-27-to-2026-05-29.md#what-to-fix--improve)
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
[`process-tooling-survey-2026-05-29.md § Pick C`](../dev-notes/archive/2026-Q2/process-tooling-survey-2026-05-29.md#pick-c--orchestrator-hygiene-queue-staleness-script-a--adr-registry-atomic-write-contract-b--pending-verifications-ledger-c):
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

> **Architect M-T1 pass — 2026-05-29.** Pick C Wave 1 parallel block
> with `queue-staleness-reconciliation` + `adr-registry-atomic-lint`
> siblings. Both operator-decide Qs ratified on the Recommended
> DURABLE path (fast-skip per T-LED-T1.1). No new ADR (confirms
> analyst `adrs_added = []`). Python stdlib only.

### Ground-truth reconciliation (READ THE LEDGER FIRST — three findings the brief did not have)

The brief and tasks were authored against an assumed ledger state
("1 FAILED row at 1 day age + 4 done rows"). The **actual** live ledger
at `spec/dev-notes/operator-side-pending-ledger.md` (read 2026-05-29 at
commit `be3050a`/`d0edc12`) does NOT match that assumption. Three
load-bearing findings that the design corrects:

- **F1 — The Pending table is EMPTY.** Lines 30-31 are the header +
  separator, immediately followed by line 32 `## Done recipes` — zero
  data rows, and **no blank line between the separator and the next
  heading**. The Bug #64 D.1.1 row was MOVED to the Done table (line
  36), not left as a FAILED Pending row. So the live table counts are
  **0 pending / 7 done / 0 cancelled**, NOT "1 FAILED + 4 done".
  Consequence: on current `main`, the lint MUST exit 0 with zero output
  (no FAILED rows exist to escalate, all Done rows have completion
  dates). T-LED-FINAL.1's "1 FAILED row at 1 day age" precondition is
  STALE — tester reconciles per D-LED-8 below. This is itself the
  feature working as intended: the ledger is currently clean.
- **F2 — `FAILED` and `fix-in-flight` appear ONLY in the Changelog
  prose, never in a live `Status` cell.** Brief §(a) and the M-T1
  directive both say "this session used FAILED and fix-in-flight too."
  True — but in the **Changelog** (lines 53-55: "Row updated FAILED →
  fix-in-flight"), describing churn that has since resolved to a Done
  row. The lint parses TABLE rows, not Changelog prose, so it never
  sees these. The canonical-enum reconciliation (D-LED-3) records
  `fix-in-flight` as a **non-canonical churn state** — if it ever
  appears in a live Status cell it is a hard schema violation.
- **F3 — Only the Pending table has a `Status` column.** The Done table
  (`Date surfaced | Recipe | Cost | Completed | Outcome`) and Cancelled
  table (`Date surfaced | Recipe | Cost | Cancelled | Reason`) encode
  status **by which table the row lives in** — there is no per-row
  Status cell. Consequence: the status-enum check, stale-FAILED check,
  and Q-LED-NOTE dev-note-citation check apply **only to the Pending
  table**. Done-table rows are checked for completion-date presence
  (R1.4.d); Cancelled-table rows are checked for cancel-date presence
  only. This is the single most important parser-shape decision and
  drives D-LED-2/D-LED-3/D-LED-5.

### Operator-decide ratifications (T-LED-T1.1)

| Q | Recommended | Architect verdict | Note |
|---|---|---|---|
| **Q-LED-WHEN** | (a) session pre-flight only | **RATIFIED — (a)** | Stale-FAILED escalation is a session-start surface, not commit-time. Rolls under the bundle's shared pre-flight pattern (queue-staleness owns the invocation example). Option C (both) is v0.2.0+. |
| **Q-LED-NOTE** | (a) require dev-note citation on FAILED rows | **RATIFIED — (a)** | FAILED Pending rows MUST cite a `spec/dev-notes/*.md` follow-up in the Notes cell. Hard violation if missing (D-LED-5). Structurally enforces the investigation-on-failure pattern Bug #64 D.1.1 set the precedent for. |
| **Q-HYG-EMIT** (bundle) | (a) markdown table + per-violation context | **INHERITED — (a)** | Diff output is a markdown table (D-LED-6). Same dialect as the two siblings. |

No amendments to either Q. Both default-path; fast-skip honoured.

### D-LED-1 — Script location, invocation, exit-code contract (hard vs soft split)

- **Path**: `scripts/operator_ledger_check.py`. Sibling to
  `scripts/spec_brief.py` / `scripts/spec_lint.py` / `scripts/hash_report.py`.
- **Shebang + PEP-723 header**: `#!/usr/bin/env python3` then the
  `# /// script` / `# requires-python = ">=3.11"` / `# ///` block,
  matching `spec_brief.py`. Stdlib only: `re`, `pathlib`, `datetime`,
  `argparse`, `sys`. **No pip / requirements.txt / virtualenv.** (Library
  checklist: stdlib-only → single-binary-friendly trivially; no system C
  deps; no Cargo touch; no stdlib-shadow concern for a `scripts/` file.)
- **Invocation**: `python3 scripts/operator_ledger_check.py` from repo
  root. The ledger path is resolved relative to the script
  (`Path(__file__).resolve().parent.parent / "spec/dev-notes/operator-side-pending-ledger.md"`),
  matching the `REPO_ROOT` idiom in `spec_brief.py`, so cwd does not
  matter.
- **CLI args** (all optional):
  - `--today YYYY-MM-DD` — override the "today" reference for the
    stale-FAILED age computation. **DEFAULT** when omitted:
    `datetime.date.today()`. See D-LED-4 — the override is the
    determinism lever and is REQUIRED by the self-test.
  - `--ledger PATH` — override the ledger path (used by the self-test
    to point at fixtures). Default = the resolved repo path above.
  - `--self-test` — run the embedded self-test suite and exit (see
    D-LED-7). Chosen over a separate `scripts/tests/` file per H2
    (keep it ≤ 200 + ~50 LoC, one file, no test-discovery harness).
- **Exit-code contract** (R1.5), with the **hard vs soft split** the
  M-T1 directive asks D-LED-1 to decide:

  | Exit | Class | Examples | Output sink |
  |------|-------|----------|-------------|
  | **0** | clean OR soft-only | clean ledger; OR only within-window FAILED soft-warnings | stdout (soft-warn) or silent |
  | **1** | HARD schema/contract violation | bad status enum; wrong column count; unparseable `Date surfaced`; Done row missing completion date; FAILED row missing dev-note citation (Q-LED-NOTE); **stale-FAILED ≥ 7 days** | stderr markdown table |
  | **≥ 2** | script failure | ledger file missing; markdown structurally unparseable (no recognizable table headers); bad `--today` arg | stderr `error:` prefixed with file:line |

  **Hard (exit 1)**: status-enum violation, column-count mismatch, bad
  date, missing completion date, **missing dev-note citation on a FAILED
  row**, and **stale-FAILED ≥ 7 days**. Rationale: per the verdict tree,
  stale-FAILED IS the primary value class — it must fail the pre-flight
  loudly so the orchestrator surfaces it. The K3 mitigation calls the
  escalation a "SOFT warning (operator can override)" — that is reconciled
  here as: it exits 1 (visible, non-silent) but the operator MAY override
  the pre-flight gate (it is not a CLAUDE.md REGRESSION-class block). The
  exit code is the signal; override is an operator action, not a code path.
  **Soft (exit 0 + stdout line)**: a FAILED row that is WITHIN the 7-day
  window emits an info-level "carry-over" line to **stdout** (not stderr)
  and does NOT fail (R2.5). This is the carry-over-status report the
  orchestrator pastes at session start.

### D-LED-2 — Ledger table parser (3 shapes; fragility flagged)

The parser is the single biggest fragility surface (falsifier K1). Design
for **markdown-tolerant, schema-keyed-by-heading** parsing:

- **Table discovery by heading anchor, NOT by position.** Walk the file
  line-by-line; a `## ` heading whose text matches one of three known
  anchors switches the active table context:

  | Heading (normalized, case-insensitive, strip trailing parenthetical) | Table key | Required columns (in order) | Status col? |
  |---|---|---|---|
  | `Pending recipes` | `pending` | `Date surfaced, Recipe, Cost, Unblocks, Status, Notes` | **YES** (index 4) |
  | `Done recipes` | `done` | `Date surfaced, Recipe, Cost, Completed, Outcome` | no |
  | `Cancelled recipes` | `cancelled` | `Date surfaced, Recipe, Cost, Cancelled, Reason` | no |

  Heading match strips the `(audit trail)` suffix and is case-insensitive.
  Any `## ` heading NOT in this set (e.g. `Conventions`, `Changelog`)
  switches context to `None` — rows under it are ignored. This is what
  makes the parser robust to F1 (no blank line before the next heading):
  the heading itself terminates the current table.

- **`SCHEMA` named constant** (K2 mitigation, T-LED-T1.3). Module-top
  dict, the single change-management surface:
  ```python
  SCHEMA = {
      "pending":   {"heading": "pending recipes",
                    "columns": ["Date surfaced", "Recipe", "Cost",
                                "Unblocks", "Status", "Notes"],
                    "status_col": "Status",
                    "date_col": "Date surfaced",
                    "completion_col": None},
      "done":      {"heading": "done recipes",
                    "columns": ["Date surfaced", "Recipe", "Cost",
                                "Completed", "Outcome"],
                    "status_col": None,
                    "date_col": "Date surfaced",
                    "completion_col": "Completed"},
      "cancelled": {"heading": "cancelled recipes",
                    "columns": ["Date surfaced", "Recipe", "Cost",
                                "Cancelled", "Reason"],
                    "status_col": None,
                    "date_col": "Date surfaced",
                    "completion_col": "Cancelled"},
  }
  ```
  `columns` is the REQUIRED, ordered prefix. Per K2 the parser asserts
  `len(cells) >= len(required)` and that the header row's first
  `len(required)` cells match (case-insensitive, trimmed) — extra
  trailing columns are accepted (forward-compat for new optional
  columns). A header-row mismatch is a HARD violation
  (`schema-table-header`) — it means the table shape drifted, which is
  the intended structural signal.

- **Row tokenization** (handles K1 fragility):
  1. A data row is any line that, after `strip()`, starts AND ends with
     `|` and is NOT the `|---|---|` separator (separator = every cell is
     empty or all-dashes/colons after trim).
  2. Split on **unescaped** pipes: split on the regex `(?<!\\)\|`, then
     `.replace("\\|", "|")` per cell to un-escape, then `.strip()` each
     cell. Drop the leading/trailing empty cells produced by the bounding
     pipes.
  3. **Markdown-strip helper** `normalize_cell(s)` — used ONLY for the
     status-enum match and the date-parse, NEVER for storing the raw cell
     (the raw cell is preserved for the diff `observed` column):
     - strip surrounding `**`/`__` (bold), `*`/`_` (italic), backticks;
     - collapse markdown links `[text](url)` → `text`;
     - `.strip()`.
     Applied via small regexes; e.g. the Bug #64-style
     `**FAILED 2026-05-29**` normalizes to `FAILED 2026-05-29`, from which
     the status token is the first whitespace-delimited word, upper/lower
     normalized against the enum (D-LED-3).
  4. **Multi-line Notes cells** (K1): the live ledger keeps each row on a
     **single physical line** (verified — line 36 is one very long line).
     v0.1.0 parses **one row per physical line** and does NOT attempt
     multi-physical-line cell continuation. **FRAGILITY FLAG**: if a
     future operator row wraps a Notes cell across physical lines, the
     wrapped lines would be mis-tokenized. Mitigation: the parser detects
     a "row" line whose cell count is `< len(required)` and emits a HARD
     `schema-row-truncated` issue (rather than silently mis-parsing),
     pointing the operator at the offending file:line. Multi-physical-line
     cell support is an explicit **v0.2.0 candidate** (documented in K1).
     This is the deliberate fragility/robustness trade for ≤ 200 LoC.

- **Empty table is valid.** A table heading followed by header+separator
  and zero data rows (F1, the current Pending table) is clean — yields an
  empty row list, no violations.

### D-LED-3 — Status enum + canonical reconciliation (T-LED-T1.4 partial; K2)

- **Canonical enum** (named constant, applies ONLY to the Pending table's
  `Status` cell per F3):
  ```python
  CANONICAL_STATUS = {"pending", "FAILED", "done", "cancelled"}
  ```
  Match is: `normalize_cell(status_cell)` → take the first token →
  case-fold compare against a case-insensitive view of the enum, BUT the
  canonical stored form preserves the brief's exact casing (`FAILED`
  upper, others lower). A cell whose first token does not map to the enum
  is a HARD `schema-status-enum` violation.
- **Canonical-enum reconciliation (the M-T1 directive's required output)**:

  | Value | Canonical? | Where seen | Lint behaviour |
  |---|---|---|---|
  | `pending` | YES | conventions, brief | accept |
  | `done` | YES | conventions, brief | accept (in Pending table — a row about to move to Done) |
  | `cancelled` | YES | conventions, brief | accept |
  | `FAILED` | YES (added at v0.1.0) | brief §(a), Bug #64 changelog | accept; triggers stale + Q-LED-NOTE checks |
  | `fix-in-flight` | **NO — non-canonical churn state** | Changelog prose only (line 54), never a live cell (F2) | HARD `schema-status-enum` if it appears in a live Status cell. Documented as a finding: it is an *informal transitional* state the orchestrator used in prose; it is NOT promoted to the enum at v0.1.0. If the operator wants it canonical, that is a v0.1.x SCHEMA bump (K2 change-management signal). |
  | `blocked`, `in-progress`, anything else | NO | hypothetical | HARD `schema-status-enum` |

  **Recommendation surfaced to operator**: do NOT add `fix-in-flight` to
  the enum at v0.1.0. Rationale — it is a transient state that always
  resolves to `done`/`cancelled`/`FAILED` within a session; promoting it
  would weaken the "FAILED rows must cite an investigation" contract
  (a row could sit in `fix-in-flight` indefinitely with no escalation).
  If operator feedback in the first 2 weeks shows a legitimate need, add
  it as a v0.1.x SCHEMA + enum bump.

### D-LED-4 — Stale-FAILED detection + the "today" determinism lever (T-LED-T1.4)

- **Named constant**: `STALE_FAILED_DAYS = 7` (module-top, K3 tunable).
- **"Today" source** — CRITICAL given the future-dated test env
  (system clock = 2026-05-29):
  - **Production default**: `datetime.date.today()` (real clock) when
    `--today` is omitted.
  - **Determinism override**: `--today YYYY-MM-DD` parsed via
    `datetime.date.fromisoformat`. **The self-test (D-LED-7) and the
    falsification probe (P-LED-1) MUST pass `--today` explicitly** so the
    age computation is reproducible and not coupled to the wall clock.
    **RECOMMENDED** (and required for testability) — without it, a test
    asserting "8 days old fires, 1 day old does not" would silently rot
    as the real date advances.
  - Bad `--today` (unparseable) → exit ≥ 2 (`error: --today must be ISO
    YYYY-MM-DD`).
- **Age rule**: for each Pending row with normalized status `FAILED`,
  `age_days = (today - date_surfaced).days`. The age is measured from
  **`Date surfaced`** (the only stable per-row date in the Pending
  table; there is no separate "FAILED-since" column at v0.1.0 — a
  documented approximation. K3 note: if a row sits pending for days
  THEN goes FAILED, age is overstated — conservative, escalates
  earlier, which is the safe direction).
  - `age_days >= STALE_FAILED_DAYS (7)` → HARD `stale-failed` issue,
    exit 1, escalation reminder in the diff.
  - `0 <= age_days < 7` → SOFT `failed-within-window` carry-over line on
    stdout, exit unaffected (R2.5).
  - `age_days < 0` (Date surfaced in the future relative to `--today`) →
    HARD `schema-future-date` (defensive; a future surfaced-date is a
    data error).

### D-LED-5 — Done-row completion-date + FAILED-row dev-note citation (Q-LED-NOTE lock; R1.4.d/.f)

- **Done-row completion-date (R1.4.d)**: for every row in the **done**
  table, the `Completed` cell (index 3) must be a non-empty ISO date
  parseable by `datetime.date.fromisoformat` after `normalize_cell`.
  Missing/empty/unparseable → HARD `missing-completion-date`. (Cancelled
  table: the `Cancelled` cell is checked the same way under issue class
  `missing-cancel-date` — symmetric, cheap, and matches R4.1's
  "exclude-rule on the Cancelled table (no *completion* date required)":
  Cancelled rows are NOT required to have a *completion* date; they ARE
  required to have a *cancel* date, which is the analogous field.)
- **FAILED-row dev-note citation (Q-LED-NOTE = a, R1.4.f)**: for every
  Pending row whose normalized status is `FAILED`, the **raw** `Notes`
  cell must contain at least one substring matching the regex
  `spec/dev-notes/[A-Za-z0-9._\-/]+\.md` (matched against the raw cell so
  links inside `[...](...)` still match the path text). Missing → HARD
  `missing-devnote-citation`, exit 1. This check runs INDEPENDENTLY of
  the stale check — a 1-day-old FAILED row with no citation still hard-fails
  (the citation is required from row creation, not after 7 days). This is
  exactly P-LED-1's "missing-citation hard" assertion.

### D-LED-6 — Output format (inherits bundle Q-HYG-EMIT markdown dialect; R2)

- **HARD issues (exit 1)** → **stderr**, a single markdown block:
  ```text
  operator-ledger-check: <N> issue(s) detected
  | issue | row | observed | expected | action |
  |-------|-----|----------|----------|--------|
  | stale-failed | 2026-05-20 Foo recipe | FAILED, surfaced 2026-05-20 (9 days old) | resolve or cancel within 7 days | escalate to analyst OR mark cancelled |
  | missing-devnote-citation | 2026-05-28 Bar recipe | FAILED, Notes has no spec/dev-notes/*.md | a follow-up dev-note path | add investigation dev-note link to Notes |
  | schema-status-enum | 2026-05-27 Baz recipe | status: "blocked" | one of {pending, FAILED, done, cancelled} | normalize the Status cell |
  | missing-completion-date | 2026-05-26 Qux populate | Completed cell empty | ISO YYYY-MM-DD | fill the Completed cell |
  ```
  Columns: `issue` (the issue class), `row` (`Date surfaced` + first ~40
  chars of the Recipe cell, markdown-stripped), `observed`, `expected`,
  `action`. Exactly the bundle Q-HYG-EMIT dialect the two siblings use,
  so the orchestrator composes ONE drift block.
- **SOFT within-window FAILED (exit 0)** → **stdout**, a separate
  one-line-per-row block (R2.5):
  ```text
  operator-ledger-check: 1 carry-over (within 7-day window, not escalated)
  - 2026-05-28 Foo recipe — FAILED, surfaced 2026-05-28 (1 day old; escalates 2026-06-04)
  ```
  The `escalates <date>` is `date_surfaced + 7 days` — actionable for the
  orchestrator's session header.
- **Clean run** → ZERO output, exit 0 (R2.3).
- **Script failure (exit ≥ 2)** → stderr `error: <msg>` with the ledger
  path and, where available, the offending line number (R2.4).

### D-LED-7 — Embedded self-test (`--self-test`; R4.1/R4.2; T-LED-T1.7)

Inline `--self-test` flag (chosen over a separate `scripts/tests/` file
per H2 / LoC budget; no test dir exists today). Each case writes a tiny
fixture ledger to a `tempfile.TemporaryDirectory()`, runs the check
function in-process with an explicit `--today`, and asserts the
(exit_code, issue_classes) tuple. Cases (≥ 6, covers T-LED-T1.7):

1. **clean** — empty Pending, ≥ 1 valid Done row → exit 0, no output.
2. **schema-status-enum** — Pending row `status: blocked` → exit 1,
   class `schema-status-enum`.
3. **stale-failed fires at 8 days** — Pending FAILED, `Date surfaced`
   8 days before `--today`, valid Notes citation → exit 1, class
   `stale-failed`.
4. **not-stale at 1 day** — Pending FAILED, surfaced 1 day before
   `--today`, valid citation → exit 0, soft `failed-within-window`
   line on stdout (assert stdout non-empty, exit 0).
5. **missing-completion-date** — Done row, empty `Completed` → exit 1,
   class `missing-completion-date`.
6. **missing-devnote-citation** — Pending FAILED (1 day old), Notes has
   NO `spec/dev-notes/*.md` → exit 1, class `missing-devnote-citation`
   (proves the citation check is independent of staleness).
7. **cancelled-table-exclusion** — Cancelled row with a valid `Cancelled`
   date and NO completion column → exit 0 (proves the Done-only
   completion rule does not over-reach onto Cancelled).
8. **Bug #64 D.1.1 regression (verbatim)** — the verbatim line-36 Done
   row parsed as a fixture → exit 0 (proves K1 markdown-tolerant parse on
   the real embedded-link/bold/multi-clause Notes cell; the Done table
   has no Status/citation requirement so it must parse clean).

`--self-test` runs all cases, prints `self-test: N passed` to stdout on
success (exit 0) or `self-test FAILED: <case>` to stderr (exit 1). R4.2:
sub-1-s (in-memory fixtures, no I/O beyond temp files).

### D-LED-8 — Tester precondition reconciliation (consequence of F1)

T-LED-FINAL.1 was written assuming "1 FAILED row at 1 day age + 4 done
rows." Per F1 the live ledger is **0 pending / 7 done / 0 cancelled**.
**Corrected expectation for the tester**: running
`python3 scripts/operator_ledger_check.py` on current `main` MUST exit 0
with ZERO output (no FAILED rows to escalate; all 7 Done rows carry
completion dates in the `Completed` column; no Pending rows to enum-check
or citation-check). If it does NOT exit 0/clean, that is a finding — most
likely a Done-row `Completed` cell that fails ISO parse, which the tester
surfaces. The "1 FAILED row" path is exercised by the self-test (case 3/6)
and the P-LED-1 probe, NOT by the live ledger.

### Falsification probe — P-LED-1 (T-LED-T1.8)

**Hypothesis under test**: the lint hard-fails a FAILED row that (a) lacks
a dev-note citation AND (b) is > 7 days old, flagging BOTH issue classes.

**Procedure** (READ-ONLY on the real ledger — use a fixture, NOT an edit
to the live file):
1. Create a temp ledger fixture (copy of the live ledger structure) with
   the Pending table containing ONE synthetic row:
   `| 2026-05-20 | Synthetic stale recipe | ~5 min | nothing | **FAILED** | TODO investigate |`
   (Note: `Date surfaced` 2026-05-20, Notes has NO `spec/dev-notes/*.md`.)
2. Run `python3 scripts/operator_ledger_check.py --ledger <fixture> --today 2026-05-29`.
3. **Assert**: exit code 1; the stderr markdown table contains a
   `stale-failed` row (9 days old ≥ 7) AND a `missing-devnote-citation`
   row for the synthetic recipe.
4. **Restore**: delete the temp fixture. The live ledger is never touched
   (R-NR.1). Then run with a citation added + `Date surfaced` 1 day before
   `--today` and assert exit 0 with a soft within-window line (the
   negative control).

This is also self-test cases 3 + 6 composed; P-LED-1 is the
operator-runnable falsification recipe and the tester runs it as
T-LED-FINAL.2/.3.

### R3.3 — Ledger frontmatter amendment (OWNED by this brief; K4)

Add a single `validated_by:` frontmatter line to
`spec/dev-notes/operator-side-pending-ledger.md`. The exact byte-level
edit (applied by the DEVELOPER at T-LED-D11, NOT the architect — architect
only specifies it here per the READ-ONLY-on-non-owned constraint and to
keep the frontmatter touch in ONE developer commit):

- Add to the frontmatter block, after the `updated:` line:
  `validated_by: scripts/operator_ledger_check.py  # operator-ledger-schema-lint v0.1.0`
- Add a Changelog row at the bottom of the ledger:
  `- 2026-05-29 (operator-ledger-schema-lint v0.1.0): schema now validated by scripts/operator_ledger_check.py (status enum, stale-FAILED escalation, done-row completion dates, FAILED-row dev-note citation). READ-ONLY lint; append-only contract unchanged.`
- **R-NR.5/R-NR.6**: NO table-row body bytes change. `git diff` on the
  ledger shows ONLY the frontmatter line + the Changelog row.

### R3.4 — AGENT.md cross-reference (OWNED by this brief; K4 — section decision)

**Decision**: add a **new `### Pending operator-verification ledger`
subsection nested UNDER the existing `## Queue pre-flight reconciliation
sweep` `##`-level section** (AGENT.md line 443), placed AFTER its existing
body (after line 468) and BEFORE the next `## The vibe-coding loop` (line
470).

Rationale for this choice over the two analyst options:
- The analyst offered "(A) new `## Pending operator verifications`
  top-level subsection OR (B) cross-link under § Queue pre-flight."
  Chosen = a **hybrid**: a NEW named subsection (gives the ledger its own
  anchor for the trace/cross-links) but **nested as `###` under the
  existing Queue pre-flight `##`** — because Q-LED-WHEN=(a) ties the
  ledger lint to the SAME session pre-flight surface the queue-staleness
  sweep owns. Keeping them under one `##` section means the orchestrator
  reads ONE "session pre-flight" block, not two scattered ones.
- **Collision avoidance (K4)**: the sibling `queue-staleness-reconciliation`
  OWNS the *body* of `## Queue pre-flight reconciliation sweep` (it adds
  the `scripts/queue_staleness_check.py` invocation example to the existing
  numbered steps). This brief adds a **distinct new `###` subsection AFTER
  that body** — different line range, no overlap with the sibling's edit to
  the numbered-steps list. The two edits are append-adjacent but
  non-overlapping; if both land in the same session the developer applies
  this brief's `###` block strictly after the queue-staleness body edit.
- `adr-registry-atomic-lint` OWNS `.claude/agents/architect.md` (different
  file entirely) — no AGENT.md contention.

**Exact subsection text** (applied by DEVELOPER at T-LED-D12):
```markdown
### Pending operator-verification ledger (2026-05-29 contract)

The single source of truth for operator-run recipes that survive
session boundaries is
[`spec/dev-notes/operator-side-pending-ledger.md`](spec/dev-notes/operator-side-pending-ledger.md)
(orchestrator-maintained, append-only). Its schema is enforced by
`scripts/operator_ledger_check.py` (Python stdlib; exit 0 clean / 1 on
schema violation or stale-FAILED escalation / ≥ 2 on script failure).
Run it at session pre-flight alongside the Queue-staleness sweep:

    python3 scripts/operator_ledger_check.py        # uses today's date
    python3 scripts/operator_ledger_check.py --today 2026-05-29   # deterministic

FAILED rows older than 7 days escalate (exit 1); FAILED rows within the
window emit a soft carry-over line for the session header. Every FAILED
row MUST cite a follow-up `spec/dev-notes/*.md` investigation in its
Notes cell (Q-LED-NOTE). See
[`spec/operator-ledger-schema-lint/feature.md`](spec/operator-ledger-schema-lint/feature.md).
```

### ADR confirmation

**No new ADR** (confirms analyst `adrs_added = []` and the bundle's § ADR
readiness flag). This feature is a script + two thin contract codifications
(ledger frontmatter R3.3 + AGENT.md cross-reference R3.4) in/under sections
that already exist. No anchor SHA in `spec/anchors.toml` is added or
changed (R-NR.4 all-PASS byte-identical). The ADR-registry-atomic-lint
sibling owns the architect.md amendment.

### Library checklist (D-LED dependency gate)

| Check | Verdict |
|---|---|
| Single-binary friendly | N/A (Python script; zero infra) — PASS |
| No system C deps | PASS — `re`/`pathlib`/`datetime`/`argparse`/`sys`/`tempfile` are pure stdlib |
| Edition 2024 compatible | N/A (no Rust) |
| Stdlib-shadow (Rust) | N/A (Python; `scripts/` filename does not shadow) |
| Maintained | PASS — stdlib |
| License | PASS — no new dep |
| Python deps | **ZERO** (R-NR.3) — `python3` ≥ 3.11 per PEP-723 header (matches `spec_brief.py`) |

### Open items for developer

- **F1 surprise**: the live Pending table is empty; do NOT hand-author a
  "1 FAILED row" expectation. The clean-exit-0 path is the current-main
  behaviour. The FAILED path lives in the self-test + P-LED-1 fixtures.
- **Single-physical-line row assumption** (D-LED-2 fragility flag): emit
  `schema-row-truncated` on undersized rows rather than mis-parse;
  multi-line-cell support is v0.2.0.
- **`Date surfaced` is the staleness clock** (D-LED-4): no separate
  FAILED-since timestamp at v0.1.0 — conservative (escalates early).

## Implementation

**Developer M-DEV pass — 2026-05-30. All D1-D14 tasks completed (D13 deferred to orchestrator per shared-file discipline).**

### What was built

`scripts/operator_ledger_check.py` — 350-line Python 3.11+ stdlib-only script. Key components:

- **`SCHEMA` dict** (module-top): the single change-management surface for the 3 table shapes (Pending 6-col, Done 5-col, Cancelled 5-col). Extra trailing columns accepted (K2 forward-compat).
- **`CANONICAL_STATUS`** frozenset: `{pending, FAILED, done, cancelled}`. `fix-in-flight` is non-canonical; HARD `schema-status-enum` if it appears in a live Status cell.
- **`STALE_FAILED_DAYS = 7`**: named constant (K3 tunable).
- **`parse_ledger()`**: heading-anchored 3-table parser. Switches active table context on `## ` headings (case-insensitive, strips `(audit trail)` suffix). Handles F1 (empty Pending table immediately followed by next `##` heading — no blank line). Escaped-pipe split via `(?<!\\)\|`. `normalize_cell()` strips bold/italic/backtick/links for enum+date match; raw cell preserved for diff output. Undersized rows emit HARD `schema-row-truncated` (no silent mis-parse).
- **`check_rows()`**: per-row semantic checks — Pending: status-enum, stale-FAILED (HARD >= 7d, SOFT < 7d to stdout), missing-devnote-citation (HARD, independent of staleness); Done: missing-completion-date (HARD); Cancelled: missing-cancel-date (HARD, symmetric).
- **`format_hard_table()` / `format_soft_block()`**: bundle Q-HYG-EMIT markdown dialect — HARD to stderr, SOFT to stdout, clean = zero output.
- **`run_self_test()`**: 8 inline cases using `tempfile` fixtures + explicit `--today 2026-05-29`. Sub-1-s.
- **CLI**: `--today YYYY-MM-DD` (determinism lever), `--ledger PATH` (P-LED-1 / hermetic testing), `--self-test`.

### Gates verified (developer side)

| Gate | Command | Output |
|------|---------|--------|
| 8/8 self-test pass | `python3 scripts/operator_ledger_check.py --self-test` | `self-test: 8 passed` (exit 0) |
| Live ledger clean | `python3 scripts/operator_ledger_check.py --today 2026-05-30` | exit 0, zero output |
| P-LED-1 HARD + SOFT | temp fixture, `--today 2026-05-29` | exit 1 with `stale-failed` + `missing-devnote-citation`; negative control exit 0 soft line |
| Anchors | `bash scripts/verify_anchors.sh` | `ANCHORS PASS (84 / 84)` |
| Stdlib-only | no pip deps, no requirements.txt | confirmed |

### Files changed

- `scripts/operator_ledger_check.py` — NEW (350 lines)
- `spec/dev-notes/operator-side-pending-ledger.md` — frontmatter `validated_by:` + `updated:` bump + Changelog row (zero table-row body bytes changed; R-NR.5)
- `AGENT.md` — new `### Pending operator-verification ledger (2026-05-29 contract)` subsection nested under `## Queue pre-flight reconciliation sweep`

### trace.toml update (orchestrator to flip)

`REQ-OPERATOR-LEDGER-SCHEMA-LINT-001`: `crates = []`, `tests = ["scripts/operator_ledger_check.py"]`, `state = "dev-done"` (dev-done → tester handoff).

## Verification

Tester M-FINAL 2026-05-30: VERDICT → PASS. Report at
[`spec/operator-ledger-schema-lint/reports/test-20260530-070513-v0.1.0.md`](../archive/tester-reports-2026-05-to-06.tar.gz).
All 7 gates pass: self-test 8/8, live exit 0, P-LED-1 fires both flags,
negative control clean, --today deterministic, fix-in-flight HARD, anchors
84/84, AGENT.md subsection coexists with queue-staleness sibling edit.

## Changelog

- 2026-05-29 (analyst): Feature brief authored under Pick C Wave 1
  promotion per
  [`spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`](../dev-notes/archive/2026-Q2/pick-c-orchestrator-hygiene-2026-05-29.md).
  R1-R4 + R-NR (7 clauses) + K1-K4 + H1-H4 + Q-LED-WHEN +
  Q-LED-NOTE + pre-drawn 4-cell verdict tree. Both Qs bias DURABLE
  per AGENT.md 2026-05-28. Ledger frontmatter amendment (R3.3) +
  AGENT.md cross-reference (R3.4) OWNED by this brief per bundle K4
  ownership-table. Trace row `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001`
  opened `proposed`. HANDOFF → architect (M-T1 + bundle parallel
  block with `queue-staleness-reconciliation` +
  `adr-registry-atomic-lint` siblings).
- 2026-05-30 (architect M-T1): § Design authored — D-LED-1..8 +
  P-LED-1 + Q-LED-WHEN/Q-LED-NOTE ratified (a)/(a) on Recommended
  DURABLE path (fast-skip) + Q-HYG-EMIT inherited (a) markdown table.
  **Ground-truth reconciliation (F1/F2/F3)**: live Pending table is
  EMPTY (0 pending / 7 done / 0 cancelled) — brief's "1 FAILED + 4
  done" assumption STALE; `FAILED`/`fix-in-flight` appear only in the
  Changelog prose, never a live Status cell; only the Pending table
  has a Status column (Done/Cancelled encode status by table). Hard
  vs soft exit split locked (stale-FAILED + missing-citation +
  enum/column/date/completion = HARD exit 1; within-window FAILED =
  SOFT exit 0 stdout). `SCHEMA` + `CANONICAL_STATUS` + `STALE_FAILED_DAYS
  = 7` named constants. `--today` determinism override (REQUIRED by
  self-test + P-LED-1 in the future-dated 2026 clock env). `fix-in-flight`
  recorded as NON-canonical churn state (HARD if it appears live;
  v0.1.x SCHEMA-bump if operator wants it). R3.4 AGENT.md section
  DECIDED: new `### Pending operator-verification ledger` nested under
  `## Queue pre-flight reconciliation sweep`, appended after its body
  to avoid collision with queue-staleness sibling's numbered-steps
  edit. No new ADR (confirmed). Frontmatter draft → arch-done, owner
  analyst → developer. HANDOFF → developer.
