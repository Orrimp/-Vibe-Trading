---
title: Pick C — Orchestrator hygiene compounder strategic direction
date: 2026-05-29
authors: [analyst]
status: direction
tags: [strategy, process, tooling, route-c, hygiene, bundle, compounder]
related:
  - docs/dev-notes/process-tooling-survey-2026-05-29.md
  - docs/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md
  - docs/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md
  - docs/dev-notes/post-v3-strategy-direction-2026-05-29.md
  - docs/dev-notes/weekly-retro-2026-05-27-to-2026-05-29.md
  - docs/dev-notes/operator-side-pending-ledger.md
  - spec/queue-staleness-reconciliation/feature.md
  - spec/adr-registry-atomic-lint/feature.md
  - spec/operator-ledger-schema-lint/feature.md
  - .claude/agents/architect.md
  - AGENT.md
  - spec/backlog.md
---

# Pick C — Orchestrator hygiene compounder strategic direction

> **Strategic dev-note, NOT a feature brief.** Frames the bundle
> rationale, sequencing, acceptance, risks, and operator-decide list
> for three Top-5 process-hygiene candidates the architect's
> [`process-tooling-survey-2026-05-29.md § Pick C`](process-tooling-survey-2026-05-29.md#pick-c--orchestrator-hygiene-queue-staleness-script-a--adr-registry-atomic-write-contract-b--pending-verifications-ledger-c)
> framed under "orchestrator pre-flight + write-time contracts that
> prevent recurring reactive cleanup cycles." Three feature briefs
> (`queue-staleness-reconciliation`, `adr-registry-atomic-lint`,
> `operator-ledger-schema-lint`) get authored alongside and promoted
> Queue → Active under this direction. Mirrors the Pick A trifecta
> (commit `0cea301`) and Pick B duo (commit `[pick-b]`) bundle pattern.

## § Why bundle these three — same class, three audit cycles paid for reactively

Per [`process-tooling-survey-2026-05-29.md § Pick C`](process-tooling-survey-2026-05-29.md#pick-c--orchestrator-hygiene-queue-staleness-script-a--adr-registry-atomic-write-contract-b--pending-verifications-ledger-c):
**Queue-staleness reconciliation (A) + ADR-registry atomic-write
linting (B) + operator-side pending ledger schema (C)** all target
the SAME failure class: _"recurring 30-second-per-session orchestrator
drag where a hand-maintained register-of-truth (Queue / ADR README /
ledger) drifts from its source-of-truth siblings (feature.md
frontmatter / ADR files on disk / operator-side reality), and the next
audit pays the catch-up cost reactively."_ Three audits in three weeks
caught the same shape:

| Audit date | Drift class caught | Reactive cost |
|------------|--------------------|---------------|
| 2026-05-07 | Empty orphan folder + ADR registry drift (ADRs 0044+) | ~30 min audit + manual catch-up |
| 2026-05-27 | Empty orphan folder finding (resolved 2026-05-29) | ~15 min |
| 2026-05-29 | Queue staleness (3 stale "moved to Active" stubs) + ADRs 0045-0049 unregistered + ledger formalization gap | ~45 min cohort cleanup |

Per the [`weekly-retro-2026-05-27-to-2026-05-29 § What to fix /
improve`](weekly-retro-2026-05-27-to-2026-05-29.md#what-to-fix--improve)
findings (items #1 / #5 / #6), all three got **proposed-fix**
status with concrete recipe sketches. Pick C operationalises all three
in one cohort under the durable contract instead of letting each
re-surface at the next audit.

**Durable-over-quick framing per AGENT.md 2026-05-28.** Ship the trio
under one strategic direction so:

1. The shared **"orchestrator pre-flight contract"** maturity ladder is
   ratified ONCE in this dev-note (Queue-staleness fires on session
   start; ADR-registry lint fires on commit touching `_bmad-output/planning-artifacts/architecture/decisions/`;
   ledger lint fires on commit touching `docs/dev-notes/operator-side-pending-ledger.md`
   OR as a pre-flight cron). Sequencing into three separate cycles
   would mean re-deciding the pre-flight-vs-post-commit-vs-CI split
   three times.
2. The shared **"non-zero exit on drift, operator-friendly diff
   message"** output shape gets one operator-decide pass, not three.
   Each script speaks the same dialect to the orchestrator.
3. **Maintenance follow-through**: once all three ship, future
   hygiene-class candidates (e.g. "presentation deck approval
   ledger lint" or "feature.md frontmatter status state-machine
   linter") have a Queue lane labeled `orchestrator-hygiene-compounder
   follow-ons` to plug into. The hygiene-class lint pattern is itself
   the durable artifact.

**The anti-pattern to avoid.** Ship Queue-staleness alone first
(~1 day, "feels like enough"); audit 2026-06-XX catches another ADR
drift; ship ADR-registry lint reactively two weeks later; ledger lint
gets dropped because "nothing forced it" until session-boundary
context loss bites again. Three reactive cleanup cycles for what is
structurally one bundle decision. Operator-friendly diff message
shapes drift across the three (Queue prints a table, ADR prints JSON,
ledger prints markdown) — orchestrator wraps each in a separate
prose envelope. Bundle pattern eliminates this.

The bundle framing is **NOT "ship all three in one PR"** — it's
"design all three under one direction so the pre-flight contracts mesh
and the operator-facing diff output is consistent." Sequencing below.

## § Sequencing — all three parallelize independently

Per the AGENT.md § Parallelism rules conflict matrix:

| Pair | Same file? | Same Cargo.toml? | Same artifact? | Same operator-decide Q? | Verdict |
|------|------------|-------------------|----------------|---------------------------|---------|
| `queue-staleness-reconciliation` × `adr-registry-atomic-lint` | NO (`scripts/queue_staleness_check.py` NEW vs `scripts/adr_registry_check.py` NEW) | NO (Python stdlib only for both; no Cargo touch) | NO (different orchestrator pre-flight surfaces — session-start vs commit-time) | NO (each has own emit-format + scope Qs; bundle dev-note carries shared diff-message-shape default) | **PARALLEL SAFE** |
| `queue-staleness-reconciliation` × `operator-ledger-schema-lint` | NO (`scripts/queue_staleness_check.py` NEW vs `scripts/operator_ledger_check.py` NEW) | NO | NO (different session-start probes) | NO | **PARALLEL SAFE** |
| `adr-registry-atomic-lint` × `operator-ledger-schema-lint` | NO (`scripts/adr_registry_check.py` NEW vs `scripts/operator_ledger_check.py` NEW) | NO | NO (different markdown lint targets) | NO | **PARALLEL SAFE** |
| Any × in-flight agents (Bug #64 dev, v5 presenter, v2.1 presenter, v5 cleanup dev) | NO (Bug #64 dev edits trace.toml + ledger ROW only; v5 presenter writes decks; v2.1 presenter writes decks; v5 cleanup dev touches `crates/strategy/` only — none touch `scripts/` NEW files OR add hygiene-lint contracts at AGENT.md / architect.md) | NO | NO | NO | **PARALLEL SAFE** |

So architect M-T1 passes can spawn concurrently for all three promoted
briefs. The orchestrator should kick architect on all three in the
same tool-use block — same parallel-spawn pattern Pick A used for
visual-fail-html-reporter + viewport-matrix and Pick B used for
redactor + asserter.

### Wave 1 (NOW, parallel)

All three promoted features run in parallel. Cost summary per the
process-tooling-survey:

| Feature | Investment | Wall-clock |
|---------|------------|------------|
| `queue-staleness-reconciliation v0.1.0` | ~1 dev day + ~0.25 tester day | ~1.5 days |
| `adr-registry-atomic-lint v0.1.0` | ~0.5 dev days + ~0.25 tester day | ~1 day |
| `operator-ledger-schema-lint v0.1.0` | ~0.5 dev days + ~0.25 tester day | ~1 day |
| **Bundle total** | **~2 dev days + ~0.75 tester day** | **~1.5 days wall-clock (parallel)** |

Sequential would be ~3+ wall-clock plus three operator approval
cycles; parallel is one operator approval cycle covering all three.
The cheapest of the architect's Month-1 picks (Pick A trifecta is
~5-7 days; Pick B duo is ~2 days; Pick C trio is ~2 days). High ratio
of impact-per-effort per the survey's MEDIUM × 3 items framing.

### Wave 2 (none — trio ships at v1.0)

Unlike Pick A's trifecta which has an explicit Wave 2 (harness
v0.3.0+ Recipe extensions deferred until v0.2.0 ships), the
orchestrator hygiene trio has NO Wave 2. All three ship at v0.1.0
and immediately enter operator-side pre-flight rotation. The "next
hygiene-class lint" candidate (e.g. presentation-deck-approval ledger
state machine, feature.md frontmatter status drift, anchored-report
body-SHA pre-commit guard) gets its own strategic dev-note when it
surfaces — this dev-note does not pre-position one. The shared
"diff-message-shape" default this dev-note locks (per Q-HYG-EMIT
below) covers future hygiene-lint scripts without per-script
re-decision.

## § Acceptance — what "orchestrator hygiene v1.0 SHIPPED" means

The bundle is **SHIPPED** when ALL of the following hold:

1. **`queue-staleness-reconciliation v0.1.0` SHIPPED** (operator-approved
   presentation; trace row state = `passed`). `scripts/queue_staleness_check.py`
   exists, reads `spec/backlog.md` Queue + Active sections, cross-
   references each Queue feature folder's frontmatter `status:` field,
   identifies mismatches (Queue stub but folder is `shipped`,
   `shipped (retired)`, or `deprecated`), exits non-zero on any
   mismatch with an operator-friendly diff message per Q-HYG-EMIT
   ratification. Wired into orchestrator pre-flight per AGENT.md
   § Queue pre-flight reconciliation sweep (2026-05-29 codified). The
   `scripts/spec_brief.py` neighbor pattern is the precedent (Python
   3 stdlib, no dep escalation; zero infra delta).

2. **`adr-registry-atomic-lint v0.1.0` SHIPPED** (operator-approved;
   trace row state = `passed`). `scripts/adr_registry_check.py`
   exists, asserts: (a) every ADR file under `_bmad-output/planning-artifacts/architecture/decisions/NNNN-*.md`
   has a row in the README.md table; (b) README.md frontmatter
   `updated:` was bumped in the same commit if any ADR was modified or
   added; (c) each ADR file's `status:` frontmatter is one of
   `{accepted, proposed, superseded, deprecated}`. Pre-commit hook OR
   post-commit CI per Q-ADR-WHEN ratification. Operationalises the
   architect.md ADR registry atomic-write contract (codified
   2026-05-29) per the [audit-2026-05-29](audit-2026-05-29.md) drift
   finding (ADRs 0045-0049 on disk but unregistered).

3. **`operator-ledger-schema-lint v0.1.0` SHIPPED** (operator-approved;
   trace row state = `passed`). `scripts/operator_ledger_check.py`
   exists, parses `docs/dev-notes/operator-side-pending-ledger.md`
   per a fixed table schema (date / recipe / cost / unblocks / status
   / notes columns), asserts: (a) `status` is one of
   `{pending, FAILED, done, cancelled}`; (b) `FAILED` rows older than
   7 days surface to operator with an escalation reminder; (c)
   `done` rows have a completion date in the `notes` (or a parallel
   `completed:` column per Q-LED-NOTE ratification). Wired into
   orchestrator pre-flight (next-session probe) per Q-LED-WHEN
   ratification.

4. **Shared "non-zero exit on drift" contract**. All three scripts
   speak the same dialect to the orchestrator (per Q-HYG-EMIT below):
   exit 0 = clean; exit 1 = drift detected with a one-screen
   operator-friendly diff/table; exit ≥ 2 = script failure (broken
   markdown parse, missing file). The diff message is markdown
   (not JSON) so the orchestrator can paste it verbatim into the
   session header.

5. **Shared "no new external dep" contract**. All three scripts use
   Python 3 stdlib only — same posture as `scripts/spec_brief.py` and
   `scripts/hash_report.py`. No requirements.txt. No virtualenv. Zero
   infra escalation.

6. **No new ADRs.** All three are scripts + thin contract codifications
   in AGENT.md / architect.md sections that already exist (the Queue
   pre-flight sweep + ADR atomic-write contract are codified; the
   ledger contract is codified at the file header level). Each
   feature's brief gets ONE Changelog row in this dev-note.

7. **One single bundle-level operator-decide closed**. Q-HYG-EMIT
   (this dev-note) ratifies the shared diff-message shape for all
   three scripts. Each feature's own brief has its own per-feature
   operator-decides; this dev-note carries the only shared one.

**Counter-example — not SHIPPED**: any one of the three at FAIL,
SOFT-PASS-with-deferred-rework, or shared diff-message-shape drift
(e.g. one script emits JSON, the others emit markdown — operator
ends up wrapping the JSON in a different prose envelope). The bundle
does NOT ship partial.

## § Risks

### K1 — Script over-reach blocks legitimate Queue promotions

The Queue-staleness reconciliation script greps for "moved Queue →
Active" stubs in `spec/backlog.md` and cross-references each Queue
feature folder's frontmatter `status:`. Edge cases the operator
explicitly accepts:

- A Queue stub that says "shipped 2026-MM-DD; see Recent" (the
  documented "stale-but-deliberate" state per AGENT.md § Queue
  pre-flight § step 2) — this is a CORRECT post-ship Queue
  annotation, not a drift case. Script must NOT flag it.
- A Queue entry that's intentionally `proposed` (not yet promoted)
  whose folder is `draft` — also clean.
- Stale-by-age (Queue > 30 days, no promotion) — analyst-recommended
  Q-QSR-1=(a) DURABLE answer is OUT OF SCOPE for v0.1.0 (per the
  feature brief's Q1). v0.2.0 may add this; v0.1.0 stays narrow on
  status-mismatch only.

**Mitigation**: feature.md K1 declares the explicit "allowed states"
table; v0.1.0 ONLY flags `status: shipped | shipped (retired) |
deprecated` against active Queue stubs. The script's diff message
includes per-mismatch CONTEXT (3 lines around the Queue stub) so
operator can decide whether to update Queue text or update
frontmatter. Manual override is one `# noqa: queue-staleness`-style
comment in the Queue stub for cases the script cannot reason about.

**Falsifier**: script blocks a legitimate Queue → Active promotion in
the first 2 weeks of operator use. Route back to analyst with the
specific case; v0.1.x patches the allowlist or the "stale-but-
deliberate" detector.

### K2 — Lint script false-positives on edge-case ADR formats

The ADR-registry atomic-lint script asserts (a) every ADR file has a
README row + (b) README frontmatter `updated:` bumped same-commit.
Edge cases:

- **ADR template file** (`_bmad-output/planning-artifacts/architecture/decisions/TEMPLATE.md`) — has
  the right shape (frontmatter + body) but is NOT a numbered ADR.
  Script must NOT require a README row for it.
- **Withdrawn / abandoned ADR file** in tree but with `status:
  withdrawn` (not yet in our enum but may surface) — needs
  README row but with a "WITHDRAWN" marker. v0.1.0 status enum
  is `{accepted, proposed, superseded, deprecated}`; analyst
  recommends Q-ADR-STATUS-ENUM clarification at architect M-T1.
- **Same-commit detection** — the architect contract says "in the
  same commit / same edit pass." `git diff HEAD~1 HEAD` semantics
  work for post-commit CI; pre-commit needs `git diff --cached`
  semantics (different invocation). Q-ADR-WHEN below picks the
  shape; M-T1 ratifies the exact git command.
- **Pre-existing ADR backfill** — when architect retroactively adds
  a Changelog row to ADR-NNNN without modifying body, the README
  frontmatter `updated:` may legitimately not bump (no semantic
  change). Q-ADR-AMEND covers the corner.

**Mitigation**: feature.md K2 declares the explicit
`TEMPLATE.md`-and-`README.md` skiplist. Q-ADR-STATUS-ENUM and
Q-ADR-AMEND surface the edge cases at architect M-T1; analyst
defaults are DURABLE = inclusive enum + explicit "no-op amendment"
marker pattern.

**Falsifier**: lint blocks a legitimate ADR amendment commit during
the first 2 weeks. Route back to analyst with the specific commit;
v0.1.x patches the skiplist or the amendment-detector.

### K3 — Ledger schema drift between operator-recipe shape and lint format

The pending ledger
(`docs/dev-notes/operator-side-pending-ledger.md`) ALREADY exists
(operator-side-pending-ledger.md, created 2026-05-29 by orchestrator
per retro fix-improve #5). The lint script formalizes its schema. Drift
risks:

- **Markdown table cells with embedded markdown** — e.g. the existing
  Bug #64 D.1.1 row has a multi-line `Notes` cell with embedded
  links + bold + nested timeline. Python markdown parsing of pipe-
  separated cells must handle escaped pipes (`\|`) AND embedded
  links with brackets that don't close on the cell boundary. Operator
  recipe shape evolves; lint format must accommodate without forcing
  the operator to rewrite past entries.
- **Append-only contract preservation** — the ledger frontmatter says
  "One row per recipe; append-only (mark status; never delete the
  row)." Lint must NOT enforce a max row count or row deletion; it
  enforces SCHEMA on existing rows only.
- **Status enum evolution** — `{pending, FAILED, done, cancelled}` is
  the v0.1.0 enum. The Bug #64 D.1.1 row uses `**FAILED 2026-05-29**`
  with embedded date. Parser must normalize via case-insensitive
  match + strip markdown formatting.

**Mitigation**: feature.md K3 declares the parser robustness
requirements: (a) tolerate markdown formatting in cells (strip
bold/italic/links before status enum match); (b) handle multi-line
notes cells via HTML-table-style row continuation OR explicit
table-end markers; (c) preserve append-only by NEVER overwriting the
ledger file (read-only lint). The lint emits SCHEMA-VIOLATION
markers per-row in the diff message rather than aborting on first
fail.

**Falsifier**: lint blocks an operator-side ledger row append in the
first 2 weeks. Route back to analyst with the specific row; v0.1.x
patches the parser tolerance.

### K4 — Cross-feature contract drift on AGENT.md / architect.md

All three features SHARE the AGENT.md / architect.md contract
codifications they operationalise. If the architect M-T1 passes for
all three try to amend the same section in different ways, the second
+ third PRs lose the first's amendments by silent overwrite.

**Mitigation**: this dev-note declares the AGENT.md / architect.md
amendment OWNERSHIP per feature:

- `queue-staleness-reconciliation` OWNS the AGENT.md § Queue
  pre-flight reconciliation sweep amendment (codifies the
  `scripts/queue_staleness_check.py` invocation; adds 1-line
  example + exit-code contract).
- `adr-registry-atomic-lint` OWNS the architect.md § ADR registry
  atomic-write amendment (codifies the
  `scripts/adr_registry_check.py` invocation; adds 1-line example;
  cross-references to AGENT.md if the architect picks Q-ADR-WHEN=(a)
  pre-commit hook).
- `operator-ledger-schema-lint` OWNS the AGENT.md ledger contract
  codification (currently the ledger has its own frontmatter
  conventions section but no AGENT.md cross-reference; this brief
  adds one).

The three architect M-T1 passes can run in parallel as long as each
restricts its AGENT.md / architect.md edit to its owned section.
**Falsifier**: two of the three M-T1 passes touch overlapping AGENT.md
lines. Route: architect serializes the second-arriving M-T1 with a
1-line awareness of the first; no separate brief needed.

### K5 — Lint scripts add to session pre-flight time

If session start runs Queue-staleness + ADR lint + ledger lint
sequentially, total pre-flight time scales linearly. The survey
estimated ~30 s/run combined; if reality is closer to ~90 s/run, the
operator may push back on always-on pre-flight.

**Mitigation**: the three scripts run sub-second on the current repo
size (Python stdlib parsing, no I/O beyond reading 3-5 markdown
files). If empirical pre-flight time exceeds 5 s after v0.1.0 ships,
v0.1.x patches the scripts to be incremental (only re-check
files touched since last invocation, recorded in
`target/orchestrator-hygiene-cache.json`). Operator-decide Q-HYG-TIMEOUT
defaults to "warn at 5 s, abort at 30 s" per the orchestrator-friendly
default. Falsifier: pre-flight >= 30 s on a clean session → route to
analyst with profiling output; v0.1.x adds the cache.

## § Operator-decide questions

**One bundle-level operator-decide.** Each promoted feature's own
brief has its own internal operator-decides (see Q1-Q2 in
[`queue-staleness-reconciliation/feature.md`](../queue-staleness-reconciliation/feature.md),
Q1-Q2 in
[`adr-registry-atomic-lint/feature.md`](../adr-registry-atomic-lint/feature.md),
and Q1-Q2 in
[`operator-ledger-schema-lint/feature.md`](../operator-ledger-schema-lint/feature.md)),
but the bundle-level choice is the shared diff-message shape.

### Q-HYG-EMIT — shared diff-message shape across all three scripts

**Q.** What emit format do all three scripts use for the
operator-friendly diff message on drift / lint violation?

**(Recommended — DURABLE) Option A: markdown table + per-violation
context lines.** All three scripts emit a markdown-formatted output
on stderr (or stdout depending on architect M-T1) shaped like:

```text
queue-staleness-check: 2 drift(s) detected
| slug | queue says | folder status | suggested fix |
|------|-----------|---------------|----------------|
| v25-tcn-overlay | "moved Queue → Active 2026-05-21" | shipped | update Queue text to "shipped 2026-05-22; see Recent" |
| ui-comet-eval | duplicate entry L2297 + L2488 | shipped | collapse to one row |
```

This shape:

- Renders cleanly in the orchestrator's session-header paste (markdown
  tables work in `claude` chat, in PR descriptions, in operator
  Slack).
- Per-violation rows give operator a one-click mental fix.
- Same dialect across all three scripts means the orchestrator
  composes one "drift detected" prose block, not three.
- Future hygiene-lint scripts inherit the dialect without re-decision.

**Cost.** ~0 — markdown table is the same overhead as plain prose.

**Rationale (DURABLE).** Per AGENT.md 2026-05-28: the operator's
mental model for "drift detected → orient to fix" is one model;
forcing them to learn three slightly-different output shapes
fragments it. Markdown table is the universally-readable shape that
survives copy-paste across surfaces (chat, PR, Slack, README).

**Option B (cheap fallback — REJECTED at analyst level).** JSON
output for tooling integration. **Rejected** per the orchestrator-
friendly framing: operator pastes the diff into a chat to triage,
not into a tool to parse. JSON adds a wrapping step. Future tooling
integration (e.g. a hygiene-dashboard widget) can `cat scripts/...
| jq` on a `--json` flag added at v0.2.0; v0.1.0 stays
markdown-default for human consumption.

**Option C (cheap-and-quick).** Plain prose with bullet points.
Acceptable for one script but fragments at three. Rejected for the
same dialect-fragmentation reason.

**Default**: A (Recommended DURABLE) per AGENT.md 2026-05-28.

---

**No other bundle-level operator-decide questions.** Each promoted
feature's brief carries its own per-feature operator-decides:

- `queue-staleness-reconciliation`: Q1 (stale-by-age scope at v0.1.0),
  Q2 (orchestrate report format details — table vs JSON; this is
  bounded by the bundle-level Q-HYG-EMIT but the feature carries its
  own minor variant).
- `adr-registry-atomic-lint`: Q1 (pre-commit hook vs post-commit CI),
  Q2 (same-commit detection semantics).
- `operator-ledger-schema-lint`: Q1 (FAILED rows require follow-up
  dev-note citation), Q2 (markdown emit vs JSON for tooling
  integration — bounded by bundle-level Q-HYG-EMIT).

## § What's NOT in the trio (despite looking like one)

Honest accounting — these look like Route C hygiene but fail the
bundle's "ship in a few dev-days" or "operationalises retro fix
proposal" criterion:

- **`v2x-trading-state-bus` (#1)** — large benefit IF v2 LLM lane
  re-activates; zero benefit if dormant. Not a hygiene script — a
  refactor. Defer to next v2 LLM activation per process-tooling
  survey § What's NOT a compounder. (Same rejection as Pick A + B.)
- **1Password GPG signing recipe (retro fix #2)** — a documented
  standing diagnostic, not a script + contract. Owner is "standing
  dev-note." Surface at first failed-commit event, not in this
  cohort.
- **Spec-lint orphan-feature cross-talk-budget block (retro fix #3,
  #4)** — analyst-brief shape change, not a script + contract. Owner
  is the analyst.md amendment + per-brief § R-NR block. Land
  alongside the next analyst-brief authoring session, not in this
  cohort.
- **Anchored-report body-SHA pre-commit guard** — would protect the
  CLAUDE.md § Non-negotiables anchor-immutability contract. Real
  candidate for the next hygiene-class cohort BUT v0.1.0 scope
  doesn't cover; surfaced as Month-2 candidate in the bundle
  follow-on lane.
- **Feature.md frontmatter status state-machine linter** — would
  catch `status: tester-done` rows whose anchor cascade isn't
  populated, etc. Candidate for Month-2 follow-on lane.
- **Presentation deck approval ledger lint** — would track
  "presented but not yet operator-approved" decks per the AGENT.md
  presentation gate. Candidate for Month-2 follow-on lane.

## § ADR readiness flag

Per the 2026-05-29 codified architect contract (writing ADR =
registering atomically in `architecture/adr/README.md`), **no Pick C
feature requires a new ADR.** All three are scripts + thin contract
codifications in AGENT.md / architect.md sections that already exist
or get a one-line cross-reference:

- Queue-staleness reconciliation: AGENT.md § Queue pre-flight
  reconciliation sweep already codifies the contract (2026-05-29);
  this brief OPERATIONALISES it with a script + one-line invocation
  example.
- ADR-registry atomic-lint: architect.md § ADR registry: writing =
  registering atomically already codifies the contract (2026-05-29);
  this brief OPERATIONALISES it with a script + one-line invocation
  example.
- Operator ledger schema-lint: ledger frontmatter
  (`operator-side-pending-ledger.md`) already declares conventions;
  this brief adds an AGENT.md cross-reference + lint script.

**Possible ride-along amendment**: if architect M-T1 finds the trio's
shared dialect contract warrants a single new AGENT.md stanza ("§
Orchestrator hygiene scripts: shared exit-code + diff-message-shape
contract"), that's a one-line addition under AGENT.md § Queue
pre-flight or similar — NOT a new ADR. Owner of that ride-along is
the queue-staleness M-T1 (first-arriving).

## § Cross-references

- [`process-tooling-survey-2026-05-29.md`](process-tooling-survey-2026-05-29.md) — Top-5 ranking (Pick C)
- [`pick-a-test-infra-trifecta-2026-05-29.md`](pick-a-test-infra-trifecta-2026-05-29.md) — bundle-pattern precedent (commit `0cea301`)
- [`pick-b-cross-cutting-safety-duo-2026-05-29.md`](pick-b-cross-cutting-safety-duo-2026-05-29.md) — sibling bundle (cross-cutting safety duo)
- [`weekly-retro-2026-05-27-to-2026-05-29.md § What to fix /
  improve`](weekly-retro-2026-05-27-to-2026-05-29.md#what-to-fix--improve)
  — retro fix #1 (Queue staleness) + #5 (operator pending ledger) +
  #6 (ADR registry drift)
- [`audit-2026-05-29.md`](audit-2026-05-29.md) — ADR registry drift
  finding (0045-0049 unregistered); reactive cleanup cost
- [`docs/dev-notes/operator-side-pending-ledger.md`](operator-side-pending-ledger.md) — living ledger; lint target
- [`AGENT.md § Queue pre-flight reconciliation sweep`](../../AGENT.md#queue-pre-flight-reconciliation-sweep-2026-05-29-contract) — contract codified 2026-05-29
- [`.claude/agents/architect.md § ADR registry`](../../.claude/agents/architect.md) — contract codified 2026-05-29
- [`_bmad-output/planning-artifacts/architecture/decisions/README.md`](../architecture/adr/README.md) — ADR registry table (lint target)
- [`spec/backlog.md`](../backlog.md) — promotions Queue → Active

## Closing

Pick C's durable framing is **"bundle the three orchestrator-hygiene
scripts under one strategic direction; ship parallel Wave 1; all
three speak the same markdown-diff dialect; all three exit non-zero
on drift with a one-screen operator-friendly fix."** The operator
decides nothing at the strategic level beyond Q-HYG-EMIT (Recommended
A = markdown table). Each promoted feature's brief carries the
per-feature operator-decides as usual.

The trio's pay-forward shape is what distinguishes it from Pick A's
test infra (which compounds linearly with feature count) and Pick B's
cross-cutting safety (which compounds per code change in its domain)
— the trio compounds **per orchestrator session**. Every future
session start, every future ADR-touching commit, every future
operator-recipe surfacing inherits the hygiene check. Same shape as
the audit cycles that paid for the trio reactively — the trio
forecloses those cycles preemptively.

**If-budget-tightens annotation.** If the operator wants to cut
scope, the cheapest pillar is `adr-registry-atomic-lint` (~0.5 dev
days) — high-frequency, narrow surface, codified contract already
exists. The most-deferable is `operator-ledger-schema-lint` (~0.5 dev
days) — the ledger is only 2 days old and the schema may evolve
through 2-3 more operator-recipe shapes before stabilising. The
trio's durable framing says **ship all three together**; the
fallback durable scope is **`queue-staleness-reconciliation +
adr-registry-atomic-lint` (skip ledger; defer to v0.2.0 once schema
stabilises)**. The trio recommendation stands per the
durable-over-quick contract, but the fallback is documented.

## Changelog

- 2026-05-29 (analyst): direction authored under Route C Pick C
  framing per `process-tooling-survey-2026-05-29.md` architect
  recommendation. Three Wave 1 features promoted via parallel
  feature.md + tasks.md authoring (`queue-staleness-reconciliation`,
  `adr-registry-atomic-lint`, `operator-ledger-schema-lint`). One
  bundle-level operator-decide Q-HYG-EMIT (Recommended DURABLE =
  markdown table + per-violation context lines) surfaced. Shared
  "non-zero exit + markdown-diff" contract + shared "Python stdlib
  only, no infra escalation" posture locked at bundle level. No
  Wave 2 — all three ship at v0.1.0 and immediately enter
  orchestrator-side pre-flight rotation. Mirrors Pick A trifecta
  (commit `0cea301`) and Pick B duo bundle pattern.
