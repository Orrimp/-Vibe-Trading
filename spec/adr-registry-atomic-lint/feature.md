---
slug: adr-registry-atomic-lint
version: 0.1.0
status: draft
owner: analyst
priority: P2
updated: 2026-05-29
---

# ADR-registry atomic-lint — v0.1.0

> **Pick C Wave 1 promoted feature (orchestrator hygiene compounder
> trio).** Per
> [`spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`](../dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md)
> this is one of three trio pillars (~0.5 dev days; cheapest pillar),
> biased toward DURABLE: a `scripts/adr_registry_check.py` script that
> enforces the architect.md ADR registry atomic-write contract by
> linting every commit touching `spec/architecture/adr/`.

## Why

Per the
[`weekly-retro-2026-05-27-to-2026-05-29 § What to fix / improve #6`](../dev-notes/weekly-retro-2026-05-27-to-2026-05-29.md#what-to-fix--improve)
finding: ADR-registry drift recurred across 3 audits (2026-05-07
caught ADRs 0044+; 2026-05-29 caught ADRs 0045-0049 on disk but
absent from `architecture/adr/README.md` table). The CONTRACT was
codified
[`.claude/agents/architect.md § ADR registry: writing = registering
atomically`](../../.claude/agents/architect.md) (2026-05-29):
"Registry update is NOT optional and is NOT a follow-up task.
Skipping it forces the next audit + orchestrator to chase it down
by hand."

The contract is codified at the architect-instructions level; this
brief **operationalises** the contract with a lint script that
catches the drift at commit time (pre-commit hook OR post-commit CI
per Q-ADR-WHEN). Per the
[`process-tooling-survey-2026-05-29.md § Top-5 deep-dives Rank 5`](../dev-notes/process-tooling-survey-2026-05-29.md#-top-5-deep-dives-condensed):
"ADR registry atomicity is a CI check (write ADR file → required
commit-time `architecture/adr/README.md` row). Operator already paid
reactively 3× in 3 weeks (audits 2026-05-07 / 05-27 / 05-29)."

Three layered consequences:

- **Codified-contract preservation.** The architect contract is only
  as good as its enforcement; without a lint, the next architect
  invocation may regress.
- **Frees architect cognitive load.** Architect M-T1 currently has to
  remember-and-execute the README row + frontmatter `updated:` bump
  manually. Lint converts the contract from "remembered-and-executed"
  to "structurally-enforced."
- **Sibling-architecture documentation cascade.** Other lint surfaces
  (presenter deck approval, anchored-report body-SHA guard) inherit
  the pattern from this brief's design.

Per process-tooling-survey: **MEDIUM per-cycle benefit, SMALL
investment (~0.5d), LOW maintenance**. The cheapest pillar of Pick
C; high-frequency surface (every ADR-touching commit); already-
codified contract.

## Requirements

### R1 — Script invocation + scope

- **R1.1** A new script `scripts/adr_registry_check.py` exists,
  invokable as `python3 scripts/adr_registry_check.py` from repo
  root. Sibling pattern to `scripts/spec_brief.py`,
  `scripts/spec_lint.py`, `scripts/hash_report.py` (Python 3 stdlib;
  no requirements.txt).
- **R1.2** Script asserts the following invariants on every commit
  that touches files under `spec/architecture/adr/`:
  - (a) Every ADR file `spec/architecture/adr/NNNN-*.md` (where
    `NNNN` is zero-padded 4-digit number per architect.md numbering
    rules) has a corresponding row in the README.md `## Registry`
    table.
  - (b) If any ADR file was MODIFIED or ADDED in the commit AND the
    architect.md "substantive change" rule applies (per architect.md
    § ADR registry § step 4 amendment guidance), the README.md
    frontmatter `updated:` field was bumped in the SAME commit.
  - (c) Each ADR file's frontmatter `status:` field is one of
    `{accepted, proposed, superseded, deprecated}` per
    architect.md § Format § frontmatter.
- **R1.3** Script EXCLUDES `spec/architecture/adr/TEMPLATE.md` and
  `spec/architecture/adr/README.md` itself from the per-ADR
  enforcement; these are infrastructure files, not numbered ADRs.
- **R1.4** Script's "same commit" detection semantics per Q-ADR-WHEN
  ratification:
  - Pre-commit hook mode (Option A): `git diff --cached --name-only`
    semantics; checks the staged changes for `spec/architecture/adr/`
    paths.
  - Post-commit CI mode (Option B): `git diff HEAD~1 HEAD --name-only`
    semantics; checks the most-recent-commit changes.
- **R1.5** Script exits with code 0 on clean (all invariants hold);
  code 1 on drift; code ≥ 2 on script failure.

### R2 — Operator-friendly diff output

- **R2.1** On drift, script writes to stderr a markdown-formatted
  diff message per bundle Q-HYG-EMIT ratification:
  ```text
  adr-registry-check: <N> drift(s) detected
  | invariant | file | observed | expected |
  |-----------|------|----------|----------|
  | (a) registry-row-missing | spec/architecture/adr/0049-*.md | no row in README.md table | add row to README.md ## Registry table |
  | (b) updated-not-bumped | spec/architecture/adr/0048-*.md | README.md `updated:` unchanged | bump README.md frontmatter `updated:` field |
  | (c) status-out-of-enum | spec/architecture/adr/0050-*.md | status: in-progress | set status: one of {accepted, proposed, superseded, deprecated} |
  ```
- **R2.2** Each drift row identifies the invariant (a/b/c), the
  offending file, the observed state, and the suggested fix.
- **R2.3** Clean run produces ZERO output (silent success).
- **R2.4** Failure (exit ≥ 2) produces an error-prefixed message
  with file:line context.

### R3 — Wire-up (pre-commit hook vs CI)

- **R3.1** Per Q-ADR-WHEN ratification, the script wires up as either:
  - (a) Pre-commit hook (Recommended DURABLE) — invoked via
    `.git/hooks/pre-commit` or a `pre-commit`-framework config that
    runs `python3 scripts/adr_registry_check.py --pre-commit` if
    any `spec/architecture/adr/` file is staged. Local-developer
    catches drift BEFORE commit.
  - (b) Post-commit CI check — invoked via `.github/workflows/`
    (or equivalent) on push, fails the PR check if drift detected
    on the HEAD commit.
- **R3.2** Architect M-T1 ratifies Q-ADR-WHEN. v0.1.0 ships with the
  ratified shape; v0.2.0 may add the second-mode opt-in flag.
- **R3.3** The architect.md § ADR registry atomic-write contract gets
  amended in this feature's M-T1 close with a 1-line invocation
  example. This brief OWNS this architect.md amendment per bundle K4
  ownership-table.

### R4 — Self-test (script smoke)

- **R4.1** Script has a `#[cfg(test)]`-equivalent self-test in
  `scripts/tests/test_adr_registry_check.py` (or inline
  `--self-test` flag per architect M-T1):
  - Asserts (a) catches a missing-README-row case
  - Asserts (b) catches a `updated:` not-bumped case
  - Asserts (c) catches an out-of-enum status case
  - Asserts exclude-rule fires on TEMPLATE.md and README.md
- **R4.2** Self-test runs in < 1 s; invokable via
  `python3 -m unittest scripts/tests/test_adr_registry_check.py`
  OR `python3 scripts/adr_registry_check.py --self-test`.

### R-NR — Non-regression contract

- **R-NR.1** Zero edits to any `spec/architecture/adr/*.md` from
  this brief — script is READ-ONLY on ADR files.
- **R-NR.2** Zero edits to `spec/architecture/adr/README.md` from
  the script's normal operation — REPORTS drift; does NOT auto-fix.
- **R-NR.3** Zero new Cargo.toml deps — pure Python 3 stdlib.
- **R-NR.4** Zero new external Python deps — no requirements.txt
  addition.
- **R-NR.5** `bash scripts/verify_anchors.sh` → all-PASS byte-identical
  pre/post. Pure script addition.
- **R-NR.6** No new clippy / fmt deltas.
- **R-NR.7** Pre-commit hook installation (R3.1 Option A) is
  OPT-IN per local developer — no automatic `.git/hooks/pre-commit`
  overwrite in this brief; developer adds the hook manually OR
  CI mode does the enforcement remotely.

## Falsifiers (K)

- **K1 — Edge-case ADR file shapes.** Per the bundle direction §
  Risk K2: TEMPLATE.md, README.md itself, and potential future
  `withdrawn`-status ADRs. **Mitigation**: feature.md R1.3 explicit
  skiplist; Q-ADR-STATUS-ENUM at architect M-T1 ratifies the v0.1.0
  status enum and the "withdrawn" handling pattern.
- **K2 — `git diff` semantics differ between pre-commit and
  post-commit modes.** `--cached` for pre-commit vs `HEAD~1 HEAD`
  for post-commit; semantics are NOT interchangeable. **Mitigation**:
  Q-ADR-WHEN locks the mode at v0.1.0; the script supports one mode
  + ratifies the exact git command at D-ADR-2.
- **K3 — Pre-existing ADR backfill / Changelog-only amendments.**
  When architect retroactively adds a Changelog row to an old ADR
  body without changing the ADR's semantic content, the README
  frontmatter `updated:` may NOT need to bump (no Decision change).
  Edge case. **Mitigation**: Q-ADR-AMEND surfaces the "no-op
  amendment" detection at architect M-T1. v0.1.0 default is
  "Changelog-only amendments DO bump README updated" — strictest;
  architect can opt-in to "no-op skip" via a comment marker per
  Q-ADR-AMEND=(a) DURABLE.
- **K4 — Lint blocks legitimate commits in first 2 weeks.** Per the
  bundle direction § Risk K2: false-positive on edge cases routes to
  v0.1.x patches. **Mitigation**: feature.md R2.1 diff message
  identifies the specific invariant + file so operator can quickly
  triage whether to fix the lint or amend the script.

## Hypotheses (H)

- **H1 — Script wall-clock ≤ 0.5 s on current repo.** Python stdlib
  + ≤ 60 ADR files + 1 README parse completes in well under the 5-s
  budget. Falsifier: empirical > 5 s → v0.1.x with cache.
- **H2 — Script ≤ 150 LoC.** Frontmatter parse + table parse + git
  diff invocation + invariant checks + markdown-table emit fits
  within ~120 LoC; +30 LoC argparse / self-test. Matches the
  analyst's ~0.5d estimate.
- **H3 — Zero existing tests break.** Pure script addition.
- **H4 — Script catches ≥ 1 drift in the first 2 weeks of
  use.** Given 3 audit cycles in 3 weeks pre-script (ADRs 0044+,
  0045-0049 unregistered), expected empirical catch rate is high
  in the first weeks. Falsifier: 0 drift in 2 weeks means
  architect's atomic-write contract has stabilised the behavior —
  confirms preventive value.

## Operator decisions

### Q-ADR-WHEN — Pre-commit hook vs post-commit CI

**Q.** Does the v0.1.0 lint wire up as a pre-commit hook OR a
post-commit CI check?

**(Recommended — DURABLE) Option A — pre-commit hook (opt-in via
local `.git/hooks/pre-commit` install or `pre-commit` framework).**
Catches drift LOCALLY before the commit lands. Architect's
"atomic-write" intent is satisfied at the point of commit creation
— the drift never enters the git history. Tight feedback loop.

**Cost.** ~5 LoC of hook glue + 1 paragraph of architect.md amendment
documenting the hook install recipe (`scripts/install_pre_commit_hook.sh`
sibling pattern with current `scripts/precheck.sh`).

**Rationale (DURABLE).** Per AGENT.md 2026-05-28: catch drift at the
earliest point in the loop. Pre-commit hook gives architect immediate
feedback ("you forgot to bump README updated") rather than waiting
for CI. Cheap-and-quick path (post-commit CI only) would still allow
the drift commit to land + require an amend commit + re-push. The
architect.md atomic-write contract reads as "atomic at commit
authoring time," not "atomic at CI time."

**Option B (cheap fallback — REJECTED at analyst level).**
Post-commit CI check only. Saves ~5 LoC hook glue + the install
recipe. **Rejected** per the architect.md contract semantics: CI
post-commit means the drift commit IS in history; CI failure
requires amend + force-push. Pre-commit hook prevents the drift from
existing in history at all.

**Option C (both).** Pre-commit AND CI for belt-and-suspenders.
Adds v0.2.0 scope; v0.1.0 ships with Option A only; CI mode opt-in
flag added at v0.2.0 if architect signals desire.

**Default**: A (Recommended DURABLE).

### Q-ADR-AMEND — Same-commit detection for Changelog-only amendments

**Q.** When architect amends an existing ADR's `## Changelog` section
without changing the ADR's Decision body, MUST the README.md
frontmatter `updated:` field bump in the same commit?

**(Recommended — DURABLE) Option A — YES, always bump on any ADR
file modification.** Strictest interpretation of the architect.md
atomic-write contract. Architect's habit becomes "every ADR touch
includes a README updated bump." Zero ambiguity at lint time. The
3-line README amendment is cheap compared to the cognitive load of
deciding "is this a substantive change?"

**Cost.** ~0 (the lint's invariant (b) just checks for bump on any
ADR modification; no semantic understanding needed).

**Rationale (DURABLE).** Per AGENT.md 2026-05-28: the cheap path
(allow no-op-amendment skip) requires the lint to UNDERSTAND
substantive vs trivial changes. That's a brittle semantic-parse
surface that risks K3 false-negatives (substantive change missed
because it looks small to the heuristic). Strictest interpretation
is structurally enforceable.

**Option B (cheap fallback — REJECTED at analyst level).**
Permissive — skip the README updated bump if the ADR diff is
Changelog-only. **Rejected** per K3 risk + the cognitive-load
argument: architect saves 0 seconds by skipping (the bump is one
line); lint saves 0 LoC. Net: the strictest interpretation is
strictly easier to enforce + maintain.

**Default**: A (Recommended DURABLE).

## Verdict tree (pre-drawn)

| Q-ADR-WHEN \ Q-ADR-AMEND | Q-ADR-AMEND=(a) always bump | Q-ADR-AMEND=(b) permissive |
|---|---|---|
| **Q-ADR-WHEN=(a) pre-commit hook** | **DURABLE — Recommended.** Tight feedback loop + strict atomic-write semantics; minimal lint surface; bundle-aligned. | INCONSISTENT — tight loop but loose semantics; K3 risk; reject. |
| **Q-ADR-WHEN=(b) post-commit CI only** | INCONSISTENT — strict semantics but loose timing; drift in history requires amend; not bundle-aligned. | REJECTED — both loose: drift in history + heuristic semantic parse. |

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
  R1-R4 + R-NR (7 clauses) + K1-K4 + H1-H4 + Q-ADR-WHEN +
  Q-ADR-AMEND + pre-drawn 4-cell verdict tree. Both Qs bias DURABLE
  per AGENT.md 2026-05-28. architect.md § ADR registry atomic-write
  contract amendment OWNED by this brief per bundle K4 ownership-
  table. Trace row `REQ-ADR-REGISTRY-ATOMIC-LINT-001` opened
  `proposed`. HANDOFF → architect (M-T1 + bundle parallel block
  with `queue-staleness-reconciliation` + `operator-ledger-schema-lint`
  siblings).
