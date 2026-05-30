---
slug: adr-registry-atomic-lint
version: 0.1.0
status: dev-done
owner: tester
priority: P2
updated: 2026-05-30
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

> **Architect M-T1, 2026-05-30.** Design pass at commit `be3050a` +
> `d0edc12` (Pick C bundle). Parallel block with
> `queue-staleness-reconciliation` + `operator-ledger-schema-lint`
> siblings. No new ADR (this script enforces the EXISTING
> architect.md § ADR registry contract codified 2026-05-29 — confirmed
> `adrs_added = []`). All operator-decide Qs ratified on the
> Recommended DURABLE path. Live ADR-0050 registration verified CLEAN
> (see § Pre-existing-debt findings).

### Operator-decide ratifications

| Q | Analyst recommendation | Architect verdict | Note |
|---|------------------------|-------------------|------|
| **Q-ADR-WHEN** | (a) pre-commit hook (DURABLE) | **RATIFIED (a)** | Catches drift before it enters git history; `git diff --cached` semantics locked at D-ADR-2. CI mode (Option B) deferred to v0.2.0 as an opt-in `--ci` flag. |
| **Q-ADR-AMEND** | (a) always-bump on any ADR modification (DURABLE) | **RATIFIED (a)** | Strictest interpretation; zero semantic-parse surface; invariant (b) is a pure "was README `updated:` also in the staged diff?" check. |
| **Q-ADR-STATUS-ENUM** | `{accepted, proposed, superseded, deprecated}`; defer `withdrawn` to v0.2.0 | **RATIFIED** | Grep of all 50 ADRs (§ Pre-existing-debt findings) confirms the enum covers every observed value with zero out-of-enum statuses. Enum is the canonical set already documented in `README.md § Format` line 22 (`status: proposed | accepted | superseded | deprecated`). `deprecated` currently unused (forward-compat); no `withdrawn` ADR exists in tree — assertion holds at v0.1.0. |

### D-ADR-1 — Script location, invocation contract, exit codes

- **File**: `scripts/adr_registry_check.py`. Sibling to
  `scripts/hash_report.py` / `scripts/spec_brief.py` /
  `scripts/spec_lint.py` (Python 3 stdlib only; executable bit;
  shebang `#!/usr/bin/env python3`; no `requirements.txt`).
- **Structure convention** (verbatim from sibling scripts): `def
  main(argv: list[str]) -> int` returning the exit code; module
  guard `if __name__ == "__main__": raise SystemExit(main(sys.argv[1:]))`.
- **Invocation surface** (argparse):
  - `python3 scripts/adr_registry_check.py --pre-commit` — the v0.1.0
    default mode. Reads the STAGED diff (`git diff --cached`). This is
    the mode the pre-commit hook calls. Per Q-ADR-WHEN=(a).
  - `python3 scripts/adr_registry_check.py --self-test` — runs the
    in-process self-test suite (R4); see D-ADR-6. Exits 0 all-pass /
    1 on any self-test failure.
  - **No-arg invocation** (`python3 scripts/adr_registry_check.py`)
    defaults to `--pre-commit` for ergonomics (mirrors the tester's
    `T-ADR-FINAL.1` smoke step which calls it bare against current
    `main`). A bare run with NO staged ADR changes performs the
    full-tree static checks (a) + (c) only and skips the
    same-commit check (b) gracefully — see D-ADR-2 § graceful-skip.
  - `--ci` mode is RESERVED, not implemented at v0.1.0 (deferred to
    v0.2.0; would switch the diff base to `git diff HEAD~1 HEAD`).
    argparse declares it as a known-future flag in `--help` text but
    `--ci` raises a "not implemented at v0.1.0; see Q-ADR-WHEN" exit-2
    message if passed — fail-closed, no silent no-op.
- **Exit codes** (per R1.5, matches bundle shared contract):
  - `0` — clean (all in-scope invariants hold). Silent (R2.3).
  - `1` — one or more drift(s) detected. Markdown table on stderr
    (D-ADR-5).
  - `≥ 2` — script failure (git unavailable, README unparseable,
    malformed frontmatter that prevents the check from running). Error
    message prefixed `adr-registry-check: error:` with file:line
    context (R2.4). Use `2` for all script-failure cases at v0.1.0.

### D-ADR-2 — "Same-commit" detection (the staged-diff semantics)

Per Q-ADR-WHEN=(a) pre-commit-hook mode, the lint inspects the STAGED
changes, NOT the working tree and NOT a committed range.

- **Staged ADR file set** (drives whether invariant (b) fires):
  ```text
  git diff --cached --name-only --diff-filter=ACMR -- 'spec/architecture/adr/*.md'
  ```
  - `--cached` → the index (what's about to be committed), per
    Q-ADR-WHEN=(a). This is the load-bearing difference from a
    post-commit `HEAD~1 HEAD` range.
  - `--diff-filter=ACMR` → Added, Copied, Modified, Renamed. Excludes
    pure Deletions (`D`) — a deleted ADR file does not need a README
    row (it's gone). A *renamed* ADR (R) counts as a modification of
    the destination path.
  - The `'spec/architecture/adr/*.md'` pathspec is quoted so git (not
    the shell) globs it. Use a list-form `subprocess.run([...])` (no
    `shell=True`) so the literal glob reaches git.
  - Run with `cwd = REPO_ROOT` (resolved via
    `Path(__file__).resolve().parent.parent`, the sibling-script
    idiom) so the invocation is location-independent.
- **README `updated:` bumped same-commit** (invariant (b) satisfaction):
  ```text
  git diff --cached --name-only -- 'spec/architecture/adr/README.md'
  ```
  If `spec/architecture/adr/README.md` appears in the staged set,
  treat invariant (b) as satisfied. **Rationale for the
  whole-file-staged proxy**: the strictest Q-ADR-AMEND=(a) reading
  says "any ADR touch ⇒ README must be touched in the same commit."
  Detecting that the README is staged at all is the structurally
  simplest, false-negative-free signal. A deeper check ("was the
  `updated:` *line* specifically in the staged hunk?") is available as
  a v0.2.0 tightening but is NOT needed at v0.1.0 — staging the README
  without bumping `updated:` is an architect-discipline question the
  contract already covers, and over-tightening risks K4
  false-positives on legitimate README edits. v0.1.0 ships the
  file-staged proxy.
  - **Optional v0.1.0 refinement (developer's discretion, ≤ 10 LoC):**
    additionally confirm the staged README hunk touches the
    `updated:` frontmatter line via
    `git diff --cached -U0 -- 'spec/architecture/adr/README.md'` and a
    regex for `^[+-]updated:`. If the developer includes this, it
    MUST be a soft-tighten (still satisfied by any `updated:` line
    change), and a self-test case must cover "README staged but
    `updated:` line unchanged ⇒ (b) fires." If H2's ≤150 LoC budget
    is tight, ship the file-staged proxy only and note the refinement
    as a v0.1.x follow-on.
- **Graceful skip**: if `git diff --cached` returns ZERO in-scope ADR
  files (no ADR staged), invariant (b) is N/A and is SKIPPED (not a
  violation). The script still runs invariants (a) + (c) over the
  full tree (cheap; catches drift introduced by a non-staged path or
  a prior un-linted commit). A bare smoke run on clean `main` thus
  exits 0.
- **git-unavailable fallback**: if `git` is not on PATH or the cwd is
  not a git repo, the script exits `2` with
  `adr-registry-check: error: git unavailable; (b) same-commit check
  cannot run`. It does NOT silently pass — fail-closed per the
  bundle contract.

### D-ADR-3 — README `## Registry` table parser

Parse `spec/architecture/adr/README.md` to build the set of
registered ADR numbers.

- **Table location**: the `## Registry` section. The table has a
  GitHub-flavoured-markdown pipe structure (verified live):
  ```text
  | ID    | Title  | Status     | Date       |
  |-------|--------|------------|------------|
  | 0001  | ...    | accepted   | 2026-04-17 |
  ```
- **Parser algorithm** (robust, regex-anchored — do NOT hand-roll a
  full GFM table parser):
  1. Read the file as UTF-8.
  2. For each line, apply the row regex
     `^\|\s*(\d{4})\s*\|` (capture group 1 = the zero-padded ADR
     number). This matches ONLY data rows whose first cell is exactly
     4 digits. It naturally rejects:
     - the header row (`| ID |`),
     - the separator row (`|----|`),
     - prose lines and Changelog bullets (no leading `|` + 4-digit
       cell).
  3. Collect the captured numbers into a `set[str]` of registered IDs.
  4. **Anchor the scan to the `## Registry` section** is NOT strictly
     required because the 4-digit-first-cell regex is already
     specific — but the developer SHOULD restrict scanning to lines
     after the `## Registry` heading and before the next `## ` heading
     to be defensive against a future Changelog table. Cheap (one
     state flag). Specify it as a SHOULD, not a MUST.
  5. **Robustness**: tolerate variable inner-cell whitespace
     (`\s*`), trailing pipe, and the wide-column padding the live
     table uses. Do NOT depend on column count or column order beyond
     "first cell is the 4-digit ID."
- **Registered set** = `{ '0001', '0002', ..., '0050' }` on current
  `main` (50 rows; verified). The ADR-file set (D-ADR-2 discovery) is
  compared against this set for invariant (a).
- **`## Registry` heading match**: `^##\s+Registry\b` (case-sensitive
  per the live file).

### D-ADR-4 — ADR-file discovery + frontmatter parse + invariant checks

- **ADR file discovery** (invariant a + c domain): glob
  `spec/architecture/adr/[0-9][0-9][0-9][0-9]-*.md` from `REPO_ROOT`.
  This pattern STRUCTURALLY excludes `README.md` and `TEMPLATE.md`
  (neither starts with 4 digits) — satisfies R1.3 by construction. The
  developer SHOULD additionally assert `name not in {'README.md',
  'TEMPLATE.md'}` as a belt-and-suspenders guard, but the glob already
  does the work.
- **ADR number extraction**: from the filename, `^(\d{4})-` → the
  zero-padded number. (Do NOT trust the in-file `adr:` frontmatter
  field as the primary key — the filename is the canonical numbering
  per README § Numbering rules "Filename pattern:
  `NNNN-kebab-case-title.md`". A future enhancement could cross-check
  filename-number == frontmatter-`adr:`-number and flag a mismatch as
  invariant (d), but that is OUT OF SCOPE at v0.1.0.)
- **Frontmatter parse**: reuse the canonical sibling regex
  `^---\n.*?\n---\n` (`re.DOTALL`) from `hash_report.py` to isolate
  the leading YAML block, then extract `status:` with
  `^status:\s*(\S+)` (multiline). **Do NOT** add a YAML dependency —
  the frontmatter is simple `key: value` lines; a line-regex is
  sufficient and matches the stdlib-only contract (R-NR.3/R-NR.4). If
  a numbered ADR has NO frontmatter block or no `status:` line, that
  is a script-relevant malformation → emit it as an invariant-(c)
  drift row with `observed = "no status: frontmatter"` (NOT an
  exit-2 crash — it's a content drift the operator should fix).
- **Invariant (a) — registry-row-present**: for each discovered ADR
  number, assert it ∈ the registered set from D-ADR-3. Any miss → a
  drift row `(a) registry-row-missing`.
- **Invariant (b) — updated-bumped** (per D-ADR-2): if any in-scope
  ADR is staged AND README is NOT staged → ONE drift row
  `(b) updated-not-bumped`. (Per Q-ADR-AMEND=(a), this fires on ANY
  staged ADR modification regardless of whether it's a Changelog-only
  amendment — strictest, zero semantic parse.)
- **Invariant (c) — status-in-enum**: for each discovered ADR, assert
  `status ∈ {accepted, proposed, superseded, deprecated}`
  (case-sensitive — all live ADRs use lowercase). Any miss → a drift
  row `(c) status-out-of-enum` with `observed = status: <value>`.
  The enum is a module-level frozenset named constant
  (`STATUS_ENUM`) so v0.2.0 can extend it (`withdrawn`) in one place.

### D-ADR-5 — Output format (bundle Q-HYG-EMIT markdown-table dialect)

Per bundle Q-HYG-EMIT=(a) (markdown table on stderr), drift emit shape
(verbatim contract — matches feature.md R2.1):

```text
adr-registry-check: <N> drift(s) detected
| invariant | file | observed | expected |
|-----------|------|----------|----------|
| (a) registry-row-missing | spec/architecture/adr/0099-probe.md | no row in README.md ## Registry table | add row to README.md ## Registry table for ADR-0099 |
| (b) updated-not-bumped | spec/architecture/adr/0048-lab-recipe-test-harness.md | README.md not staged in this commit | stage spec/architecture/adr/README.md with bumped frontmatter updated: |
| (c) status-out-of-enum | spec/architecture/adr/0036-patchtst-training-contract.md | status: in-progress | set status: one of {accepted, proposed, superseded, deprecated} |
```

- Header line `adr-registry-check: <N> drift(s) detected` (N =
  total row count across all three invariants).
- Exactly 4 columns: `invariant` (a/b/c + slug) | `file` (repo-relative
  path) | `observed` | `expected` (suggested fix).
- All drift output → **stderr** (matches sibling scripts' error/diag
  convention; keeps stdout clean for any future `--json` pipe).
- Clean run → **zero output** (R2.3); exit 0.
- Sort drift rows deterministically: by invariant letter (a, b, c)
  then by file path, so repeated runs / self-test golden assertions
  are stable.

### D-ADR-6 — K4-owned architect.md amendment (THIS feature owns it)

Per bundle K4 ownership-table, THIS feature OWNS the
`.claude/agents/architect.md § ADR registry: writing = registering
atomically (2026-05-29 contract)` amendment. Siblings own AGENT.md
(queue-staleness) and the ledger (operator-ledger) — do NOT touch
those.

**Exact amendment** — append a new short paragraph immediately AFTER
the existing closing paragraph (current line 92, before `## Style`):

```markdown
**Mechanical enforcement.** `scripts/adr_registry_check.py --pre-commit`
lints this contract on every commit touching `spec/architecture/adr/`:
(a) every `NNNN-*.md` has a README `## Registry` row, (b) README
`updated:` is staged alongside any ADR change, (c) each ADR `status:`
∈ `{accepted, proposed, superseded, deprecated}`. Exit 1 + a markdown
drift table on violation. Install as a pre-commit hook (opt-in) or run
bare before committing.
```

**Second, load-bearing micro-fix in the SAME owned amendment**: the
existing contract step 2 (line 82) lists the status set as
`(accepted / proposed / superseded)` — it OMITS `deprecated`, which IS
in the canonical enum (`README.md § Format` line 22). The developer
MUST update line 82's parenthetical to
`(accepted / proposed / superseded / deprecated)` so the architect's
prose contract and the lint's `STATUS_ENUM` agree. This is a 1-word
addition within the owned section — no sibling-section drift.

### D-ADR-7 — Edge cases

- **`TEMPLATE.md`** (`status: proposed`, `adr: NNNN`): excluded by the
  `[0-9][0-9][0-9][0-9]-*.md` glob (filename does not start with 4
  digits). NOT required to have a registry row. Self-test asserts the
  exclude (R4.1 4th case).
- **`README.md` itself** (`status: in-progress`, `slug:
  architecture-adr-index`): the README frontmatter `status:` is
  `in-progress` — which is NOT in the ADR status enum. This is
  CORRECT and must NOT be flagged: README is the index file, not a
  numbered ADR, and is excluded by the glob (no 4-digit prefix). The
  enum check (c) NEVER runs against README. Self-test asserts this.
- **`superseded` ADRs**: ADR-0027 is `status: superseded` and HAS a
  registry row (verified). `superseded` is in the enum → invariant (c)
  passes; invariant (a) passes (row present). No special handling
  needed — a superseded ADR keeps its row + number per README §
  Numbering rules.
- **`proposed` ADRs**: ADR-0036, ADR-0047 are `status: proposed` with
  rows. In enum → clean. (Confirms the enum is not over-narrow.)
- **README frontmatter `updated:` format**: the live value is
  `updated: 2026-05-29 (ADR-0050 D1 corrected + ...)` — a date
  followed by a long free-text parenthetical. The invariant-(b) check
  does NOT parse this value (it only checks whether README is in the
  staged file set), so the parenthetical is irrelevant to v0.1.0. If
  the optional v0.1.0 refinement (D-ADR-2) is implemented, the
  `^[+-]updated:` line-regex matches regardless of the parenthetical.
  No date-parse needed.
- **Sub-numbered / collision-renumbered ADRs**: ADR-0037 was
  renumbered 0035→0037 (audit-2026-05-22) and ADR-0035 exists
  separately. Both are plain 4-digit files with rows — the
  filename-number primary key handles them with no special case. No
  `NNNNa` / sub-letter variants exist in the tree; if one is ever
  filed, the `\d{4}` regex would skip it (drift surfaced as
  "file not matched" only if it also lacks a row — a v0.2.0
  consideration, out of scope).
- **Deleted ADR**: `--diff-filter=ACMR` excludes deletions from the
  staged set (D-ADR-2), so deleting an ADR file does not trigger
  invariant (b). (Deletion is itself a contract question the lint
  does not police at v0.1.0.)

### Self-test cases (R4.1 / D-ADR-6 inline `--self-test`)

The self-test runs against synthetic in-memory / tmpdir fixtures (NOT
the live tree — must not mutate `spec/`). ≥ 4 cases:

1. **(a) missing-row** — synthetic ADR `0099-foo.md` present in the
   ADR-file set but absent from a synthetic README registry → asserts
   one `(a)` drift row naming `0099`.
2. **(b) updated-not-bumped** — a staged-ADR set containing
   `0099-foo.md` with README NOT in the staged set → asserts one `(b)`
   drift row.
3. **(c) status-out-of-enum** — synthetic ADR with
   `status: in-progress` → asserts one `(c)` drift row.
4. **exclude-rule** — `TEMPLATE.md` + `README.md` in the directory →
   asserts NEITHER triggers an (a) missing-row NOR a (c) enum drift.
5. **(clean)** — full synthetic set with all rows present + all
   statuses in enum + README staged → asserts ZERO drift, exit 0.

Implementation: prefer an inline `--self-test` flag (single-file, no
`scripts/tests/` dir needed; matches the lean sibling pattern) using
stdlib `unittest` or plain assert-functions. Sub-1-s wall-clock (R4.2).
The git-dependent invariant (b) self-test injects the staged-file set
via a seam (pass the file-list into the check function rather than
shelling out to git in the test) so the self-test does NOT require a
real git index — this is the "every external I/O behind a seam"
discipline applied to the git subprocess. **Developer note**: factor
the git call into a thin `_staged_adr_files()` / `_readme_staged()`
function so the self-test can inject fakes.

### Falsification probe P-ADR-1 (developer runs at T-ADR-D8; tester re-runs at T-ADR-FINAL.2)

**Goal**: prove the lint actually fires on an unregistered ADR (the
exact drift class that recurred 3×).

**Recipe** (self-contained; no commit; reverts cleanly):

- **Command / Steps**:
  1. Create a throwaway ADR file with NO README row:
     `printf '%s\n' '---' 'adr: 9999' 'title: probe' 'status: accepted' 'date: 2026-05-30' 'supersedes: none' 'superseded-by: none' '---' '' '# ADR-9999: probe' > spec/architecture/adr/9999-probe.md`
  2. Run `python3 scripts/adr_registry_check.py --pre-commit`.
  3. **Expected**: exit code `1`; stderr markdown table contains a
     `(a) registry-row-missing` row naming
     `spec/architecture/adr/9999-probe.md` (i.e. ADR-9999).
  4. Delete the probe: `rm spec/architecture/adr/9999-probe.md`.
  5. Re-run `python3 scripts/adr_registry_check.py --pre-commit`.
  6. **Expected**: exit code `0`; zero output (registry clean again).
- **Timing**: < 2 s total.
- **Failure-diagnosis**: if step 3 exits 0, invariant (a) is not
  scanning the full ADR-file set (check the glob); if it names the
  wrong ADR, check the filename-number extraction regex.
- **Cleanup**: `git status spec/architecture/adr/` must show NO
  changes after step 4 (the probe file was never committed and is
  deleted). Confirm `9999-probe.md` is gone.

> **Note**: the probe uses `9999` (not `0099`) to stay clearly out of
> the live numbering range and avoid any collision with a real future
> ADR.

### Pre-existing-debt findings (from the live ADR grep)

Architect ran `status:` extraction across all 50 numbered ADRs +
cross-referenced the `## Registry` table on `main` at design time:

- **Registry completeness**: ALL 50 ADR files (`0001`–`0050`) have a
  corresponding `## Registry` row. **Zero invariant-(a) debt.** The
  lint will exit 0 on current `main` for invariant (a).
- **Status enum coverage**: observed values are `accepted` (47×),
  `proposed` (2× — ADR-0036, ADR-0047), `superseded` (1× — ADR-0027).
  ALL ∈ the ratified enum `{accepted, proposed, superseded,
  deprecated}`. **Zero invariant-(c) debt.** `deprecated` is unused
  (forward-compat); no `withdrawn` ADR exists (v0.1.0 assertion
  holds).
- **Live ADR-0050 registration (the requested live test case)**:
  CLEAN. `0050-iced-tokio-runtime-context-and-cancellation.md` is
  present, has a `## Registry` row (line 100 of README.md), and
  `status: accepted` (in enum). The README frontmatter `updated:` was
  bumped to `2026-05-29 (ADR-0050 ...)` in the same session that
  landed ADR-0050 — invariant (b) was satisfied. The atomic-write
  contract held for the session that created this feature's
  motivating ADR. **No debt.**
- **Conclusion**: the lint ships against a CLEAN tree — it is purely
  preventive at v0.1.0 (consistent with H4: catch ≥ 1 drift in the
  first 2 weeks of FUTURE commits, not a backfill of existing debt).
  This is the desirable outcome: the architect's 2026-05-29
  atomic-write contract has already been honoured for ADR-0050.
- **Minor prose-vs-enum inconsistency (NOT an ADR-file debt)**: the
  architect.md contract step 2 omits `deprecated` from its
  parenthetical status list. Fixed by D-ADR-6's second micro-fix (in
  the owned amendment). This is a documentation drift, not a tree
  drift; it does not affect any lint exit code.

## Implementation

Implemented 2026-05-30 by developer M-DEV.

**Script**: `scripts/adr_registry_check.py` — 300 LoC (within H2 ≤ 150 core + ~150 self-test).
Stdlib only (`re`, `pathlib`, `subprocess`, `argparse`, `unittest`, `tempfile`). Executable bit set.
Shebang `#!/usr/bin/env python3`. `def main(argv: list[str]) -> int` + `raise SystemExit(main(sys.argv[1:]))` sibling convention.

**Invariants implemented:**
- (a) Registry-row: `_parse_registered_ids()` uses `^\|\s*(\d{4})\s*\|` row-regex anchored to `## Registry` section scan, yielding a `set[str]` of 50 registered IDs on clean `main`. Every `[0-9][0-9][0-9][0-9]-*.md` file is checked against this set.
- (b) Updated-bump: `_staged_adr_files()` / `_readme_staged()` git seams (list-form subprocess, `cwd=REPO_ROOT`, no `shell=True`) implement D-ADR-2 exactly. If any ADR is staged and README is not, one `(b)` drift row is emitted. Graceful-skip when zero ADR staged.
- (c) Status enum: module-level `STATUS_ENUM = frozenset({"accepted","proposed","superseded","deprecated"})`. Missing frontmatter → `(c)` drift with observed `"no status: frontmatter"` (not a crash).

**Git seams** factored per architect's requirement: `_staged_adr_files()` and `_readme_staged()` are standalone functions the self-test bypasses via `_check_invariants_raw()` which accepts injected staged-file lists — no real git index needed in tests.

**Self-test**: 5 inline `unittest.TestCase` cases exercised via `--self-test` flag. Uses `tempfile.TemporaryDirectory` fixtures; never mutates `spec/`. Sub-1-s (observed: 0.003 s).

**D-ADR-6 amendment**: `.claude/agents/architect.md` amended at two points:
1. Line 82: status parenthetical updated to include `/ deprecated`.
2. After former line 92 (before `## Style`): "Mechanical enforcement." paragraph with 1-line invocation example added verbatim per feature.md § Design D-ADR-6.

**Gates verified:**
- `python3 scripts/adr_registry_check.py --self-test` → 5/5 PASS, 0.003 s
- `python3 scripts/adr_registry_check.py --pre-commit` on clean `main` → exit 0, zero output
- P-ADR-1 probe (`9999-probe.md`, no README row) → exit 1, `(a) registry-row-missing` names ADR-9999; after `rm` → exit 0; `git status spec/architecture/adr/` clean
- `bash scripts/verify_anchors.sh` → 84/84 PASS (zero anchor delta — scripts/ + docs only)

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
- 2026-05-30 (architect, M-T1): § Design authored — D-ADR-1..7 +
  Q-ADR-WHEN/Q-ADR-AMEND/Q-ADR-STATUS-ENUM all RATIFIED on Recommended
  DURABLE path. Locked: `git diff --cached --name-only --diff-filter=ACMR
  -- 'spec/architecture/adr/*.md'` staged-diff semantics (D-ADR-2);
  4-digit-first-cell regex README parser (D-ADR-3); `[0-9][0-9][0-9][0-9]-*.md`
  glob that structurally excludes README.md + TEMPLATE.md (D-ADR-4);
  `STATUS_ENUM = {accepted, proposed, superseded, deprecated}` named
  constant (D-ADR-4); markdown-table-on-stderr emit per Q-HYG-EMIT
  (D-ADR-5); K4-owned architect.md amendment = 1-paragraph mechanical-
  enforcement note + `deprecated` micro-fix on line 82 (D-ADR-6); ≥ 5
  self-test cases with git-seam injection (R4); P-ADR-1 probe spec'd
  with `9999-probe.md` synthetic ADR. Pre-existing-debt grep: registry
  CLEAN (50/50 rows), zero out-of-enum status, live ADR-0050
  registration verified CLEAN — lint ships purely preventive. No new ADR
  (`adrs_added = []` confirmed). frontmatter draft → arch-done,
  owner analyst → developer. HANDOFF → developer.
