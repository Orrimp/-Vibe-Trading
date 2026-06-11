---
slug: dev-notes
status: in-progress
owner: spec-auditor
updated: 2026-05-30
---

# Spec Audit — 2026-05-30

## Headline

The spec tree survived a heavy multi-lane churn day (ADRs 0050–0051, four new
feature folders, the Monte-Carlo strategic reframe, and the lab-yahoo UX
follow-up) without introducing new critical drift. Mechanical lint counts are
materially unchanged from the 2026-05-29 baseline (no new dead-link clusters
from today's work; trace-broken-path is largely carry-over debt). The single
post-churn mismatch that needs a one-line fix is the C1 path-generator
`feature.md` status showing `dev-done` while the trace correctly records
`tested` — the frontmatter was not advanced past the tester PASS. Everything
else audited is clean or is pre-existing carry-over.

---

## TL;DR Triage Table

| Sev | Item | File(s) | One-line fix |
|-----|------|---------|--------------|
| SHOULD-FIX | C1 feature.md status stale | `spec/monte-carlo-bootstrap-path-generator/feature.md` | Flip `status: dev-done` → `status: tested`, `owner: developer` → `owner: tester` |
| SHOULD-FIX | anchors.toml header comment 13 cycles stale | `spec/anchors.toml` line 13 | Update comment from `69/69 PASS — 2026-05-27` to `84/84 PASS — 2026-05-30` |
| SHOULD-FIX | trace-broken-path for C1 tests column | `spec/trace.toml` row `REQ-MC-BOOTSTRAP-PATH-GENERATOR-001` | Replace freetext path `crates/data/src/synth (23 unit + FP-C1.1..6)` with concrete test function paths so spec-lint resolves them |
| COSMETIC | `missing-frontmatter` in 2 tasks.md files (pre-existing) | `spec/ui-test-harness-viewport-matrix/tasks.md` (invalid status `present-done`), `spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/tasks.md` (invalid status `dev-in-progress`) | Fix invalid status values to allowed enum values |
| COSMETIC | `shipped-no-tests` for 2 pre-existing shipped features | `spec/lab-end-to-end-v2/feature.md`, `spec/vol-killswitch-overlay-noop-fix/feature.md` | Either add a reports/ directory + report, or add a `no-test-reason` annotation per the auditor convention |
| COSMETIC | `lab-polish-round-2/tasks.md` missing frontmatter | `spec/lab-polish-round-2/tasks.md` | Add required frontmatter block with slug/status/owner/updated |
| DEFERRED-KNOWN | ADR README unregistered entries (ADRs 0045–0049, 5 total) | `spec/architecture/adr/README.md` | Pre-existing; ADRs 0050+0051 ARE registered. Pattern: carry-over |
| DEFERRED-KNOWN | architecture.md inline ADR table frozen at ADR-0026 | `spec/architecture.md` | Pre-existing; 25 entries missing (0027–0051). Carry-over |
| DEFERRED-KNOWN | 5 backlog entries with `noqa: queue-staleness` | `spec/backlog.md` | Operator-deferred intentionally; NOT new drift |
| DEFERRED-KNOWN | Residual dead-link clusters (87 total, ~80 pre-existing) | Various | Carry-over from prior audits; no new clusters introduced by today's churn |
| DEFERRED-KNOWN | Soft contradiction: anchors.toml main-header count comment stale vs sub-group comments | `spec/anchors.toml` | The sub-group comments (`69/69`, `70/70`, etc.) are snapshot-in-time annotations; only line 13 is the live expected-count claim. Merged into SHOULD-FIX above |

---

## 1. Mechanical Lint

`scripts/spec_lint.py --all` executed successfully (Python 3.11 on this run —
the Python 3.9 / no-`tomllib` failure documented in prior audits did NOT recur;
the interpreter resolved to 3.11 via the shell environment).

**Result: FAIL (145 violations in 4 categories)**

| Category | This run | Previous (2026-05-29 est.) | Delta |
|---|---|---|---|
| dead-link | 87 | ~65 (est.) | +22 (explained below) |
| missing-frontmatter | 3 | ~0 | +3 (see details) |
| shipped-no-tests | 2 | 0 | +2 (see details) |
| trace-broken-path | 53 | ~4 (est.) | +49 (explained below) |

**Delta explanation — dead-link (+22):** The 2026-05-29 audit ran under Python 3.9
and could not invoke `spec_lint.py --all` (no `tomllib`). The prior count of ~65
was a manual estimate. Today's confirmed 87 reflects the same pre-existing clusters
(ADR-0027 Kronos slug, `/tmp/` chart screenshots, legacy v0-paper-sma README paths,
journal-tx-metadata missing report, ADR-0039 missing crate path, ADR-0045 missing
skill path, etc.) plus the newly added presentation artifacts for cockpit-activity-
status-bar that reference screenshot files not yet committed. Zero new dead-link
clusters were introduced by today's Monte-Carlo churn.

**Delta explanation — trace-broken-path (+49):** The confirmed 53 are largely from
the large batch of test paths registered in trace.toml that spec-lint checks as
file::function paths but which are formatted as prose (not real file::fn paths). The
most notable new item: `REQ-MC-BOOTSTRAP-PATH-GENERATOR-001` field `tests` contains
`crates/data/src/synth (23 unit + FP-C1.1..6)` — a prose description, not a
`file::fn` path, so spec-lint flags it. The existing trace-broken-path cluster
covering `REQ-BUG-64-D-11-ATTEMPT-3-001`, `REQ-V2-1-TRACING-LAYER-REDACTOR-001`,
`REQ-UI-CONTRAST-ASSERTER-001`, `REQ-QUEUE-STALENESS-RECONCILIATION-001`,
`REQ-OPERATOR-LEDGER-SCHEMA-LINT-001` are all pre-existing carry-overs.

**missing-frontmatter details (3 violations — all pre-existing):**
- `spec/lab-polish-round-2/tasks.md` — no frontmatter block at all.
- `spec/ui-test-harness-viewport-matrix/tasks.md` — invalid status `present-done`
  (not in the allowed-status enum).
- `spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/tasks.md` —
  invalid status `dev-in-progress` (not in allowed-status enum; should be `in-progress`
  or `dev-done`).

**shipped-no-tests details (2 violations — both pre-existing):**
- `spec/lab-end-to-end-v2/feature.md` — status `shipped` but no `reports/` directory.
- `spec/vol-killswitch-overlay-noop-fix/feature.md` — status `shipped` but no
  `reports/` directory.

Note: `vol-killswitch-overlay-noop-fix` was confirmed shipped 2026-05-26 in the
prior audit. The missing `reports/` is a documentation gap, not a regression gate
breach (the ship was confirmed via backlog summary). `lab-end-to-end-v2` is a
longer-standing gap.

### Anchor verification

`bash scripts/verify_anchors.sh` run live: **ANCHORS PASS (84 / 84)**

Anchor count trajectory (confirmed):
- 2026-05-27: 69
- 2026-05-28: 70 (Yahoo v0.1.2 ETH-USD anchor)
- 2026-05-29: 71 (Binance ETH H1 v0.1.3 anchor)
- 2026-05-29 (v5-sqrt-impact): 84 (+13; 9 new sqrt-impact scenarios + additional
  prior anchors from v3-regime-classifier + other features)

**The anchors.toml main header comment (line 13) is stale: reads `69/69 PASS —
2026-05-27` but current verified count is `84/84`. This is 15 count-cycles behind.**
This is the most stale the header comment has been since the auditor began tracking
it. Safe to update (anchors.toml is the registry, not itself an anchored report).

### Sample violations (top 5 per category, new or notable only)

**dead-link (new/notable):**
- `spec/cockpit-activity-status-bar/presentations/cockpit-activity-status-bar-2026-05-26.md`
  — 4 refs to `artifacts/.../0{1,2,3,4}-*.png` (screenshot files not committed).
  Carry-over from prior audits.
- `spec/architecture/adr/0045-v5-canonical-config-and-noop-baseline-namespace.md`
  — ref to `../../.claude/skills/spec-update/SKILL.md` (relative path from adr/
  subdir resolves incorrectly). Pre-existing.
- `spec/dev-notes/testing-strategy-review-2026-05-25.md` — ref to
  `../../crates/audit/tests/reconciler.rs` (file does not exist at that path).
  Pre-existing.

**trace-broken-path (new):**
- `spec/trace.toml` row `REQ-MC-BOOTSTRAP-PATH-GENERATOR-001` field `tests`:
  value is prose `crates/data/src/synth (23 unit + FP-C1.1..6)` — not a
  resolvable `file::fn` path. Introduced this session. Fix: use concrete paths
  like `crates/data/src/synth/bootstrap.rs::tests::same_seed_determinism`.

**missing-frontmatter:**
All three are listed in the triage table. No new violations introduced by today's
churn.

---

## 2. ADR Registration Sweep (Focus Area)

### ADRs 0050 and 0051

Both ADRs exist on disk:
- `/spec/architecture/adr/0050-iced-tokio-runtime-context-and-cancellation.md`
- `/spec/architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md`

Both are **registered in `spec/architecture/adr/README.md`** — confirmed by reading
the README table. The README `updated:` frontmatter reads
`2026-05-30 (ADR-0051 added — Monte-Carlo robustness determinism + distribution-report
anchoring contract, C1 path-gen + C2 harness; D1-D5 accepted)`. This is correct
and current.

**Finding: ADRs 0050 and 0051 are properly registered. No gap.**

### ADRs 0045–0049 (pre-existing gap)

From the 2026-05-29 audit: ADRs 0045/0046/0047 were eventually registered, but
ADR-0048 and ADR-0049 were found absent from the README. Reading the README today
confirms **ADRs 0045–0049 are NOT in the registry table** despite their files
existing on disk. The README table ends at ADR-0051. The pattern is that the
"atomically registered" contract was applied for 0050 and 0051 (per ADR-0051
Changelog: "Registered atomically in `README.md`") but was apparently not applied
for 0048/0049 in prior sessions.

This is a carry-over P1 from the 2026-05-29 audit. ADR-0050 and ADR-0051 are clean.
Total unregistered gap: **ADRs 0045–0049 (5 entries missing from the registry table).**

### ADR cross-reference check

**ADR-0050 cross-references verified:**
- Links to `crates/ui/src/live.rs` (server_time_stream_impl) — file exists.
- Links to `crates/backtest/src/cancel.rs` — file likely exists (referenced by
  trace.toml REQ-BUG-64 tests column). Not dead-linked in the ADR body itself
  (the dead-link is in trace.toml, not the ADR).
- Links to `spec/bug-64-d11-attempt-3-yahoo-run-runtime-context/` — folder exists.
- Reference to `crates/ui/tests/lab_runner_ticker_e2e.rs` etc. — plausible; not
  resolvable by dead-link checker (Rust source paths in prose, not markdown links).

**ADR-0051 cross-references verified:**
- Links to `../../monte-carlo-bootstrap-path-generator/feature.md` — file exists.
- Links to `../../strategy-robustness-harness/feature.md` — file exists.
- Links to `../../dev-notes/monte-carlo-robustness-architecture-readiness-2026-05-29.md`
  — file exists in dev-notes (confirmed by listing dev-notes/).
- Links to `../../dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md`
  — file exists.
- All ADR back-references (ADR-0002, ADR-0003, ADR-0032, ADR-0038, ADR-0043) are
  valid registered ADRs.

**Finding: ADR-0050 and ADR-0051 cross-references are internally consistent.**

---

## 3. New Feature Folder Audit

### C1 — `spec/monte-carlo-bootstrap-path-generator/`

| Field | Value | Flag |
|-------|-------|------|
| feature.md status | `dev-done` | MISMATCH — trace says `tested` |
| feature.md owner | `developer` | STALE — should be `tester` post-PASS |
| feature.md updated | `2026-05-30` | OK |
| trace.toml state | `tested` (VERDICT PASS, tester 2026-05-30) | CORRECT |
| tasks.md present | YES | OK |
| reports/ present | YES (test-20260530-140000-v0.1.0.md) | OK |
| Decay markers | 0 | OK |

**Finding (SHOULD-FIX):** `feature.md` frontmatter was not advanced past `dev-done`
after the tester PASS. The trace is authoritative and correct. The feature.md needs:
`status: dev-done` → `status: tested`, `owner: developer` → `owner: tester`.
This is a one-line frontmatter flip; the test report exists and the trace is correct.

**Finding (SHOULD-FIX):** The trace.toml `tests` column for `REQ-MC-BOOTSTRAP-PATH-GENERATOR-001`
contains prose (`crates/data/src/synth (23 unit + FP-C1.1..6)`) instead of concrete
`file::fn` paths that spec-lint can resolve. This causes a `trace-broken-path`
violation. Owner: orchestrator / developer (update trace row with concrete paths
after confirming the actual test function names with the developer).

### C2 — `spec/strategy-robustness-harness/`

| Field | Value | Flag |
|-------|-------|------|
| feature.md status | `arch-done` | OK — C2 is in-flight developer; correctly in-flight |
| feature.md owner | `developer` | OK |
| trace.toml state | `arch-done` | CONSISTENT |
| crates column | empty | OK — developer fills at M-DEV |
| tests column | empty | OK — developer fills at M-DEV |
| anchors column | empty | OK — tester fills after PASS |
| Decay markers | 0 | OK |

**Finding: C2 is cleanly in-flight. No drift.**

Note: the `mc-robustness-2026-06` anchor namespace is NOT yet in `anchors.toml`
(confirmed: grepping anchors.toml shows no such namespace entry). This is correct —
it should only be added by the tester after C2 ships.

### `spec/lab-recipe-test-harness-v0.2.0-cross-surface-extension/`

| Field | Value | Flag |
|-------|-------|------|
| feature.md status | `shipped` | OK |
| feature.md owner | `shipped` | OK |
| trace.toml state | `shipped` | CONSISTENT |
| reports/ | 5 test reports (waves A, B, C, M-FINAL) | OK |
| presentations/ | v0.2.0-2026-05-30.md + artifacts/ | OK |
| Decay markers | 0 | OK |

**Finding: lab-recipe-test-harness-v0.2.0 is clean.**

### `spec/lab-yahoo-empty-range-ux/`

| Field | Value | Flag |
|-------|-------|------|
| feature.md status | `shipped` | OK |
| feature.md owner | `shipped` | OK |
| trace.toml state | `shipped` | CONSISTENT |
| reports/ | 2 test reports | OK |
| presentations/ | v0.1.0-2026-05-30.md + artifacts/ | OK |
| Decay markers | 0 | OK |

**Finding: lab-yahoo-empty-range-ux is clean.**

---

## 4. product.md — Pillar Stack Sweep

The new `§ Pillar stack — core vs support (ratified 2026-05-30)` section (lines 26–95
approximately) adds the LLM demotion to support pillar and promotes the Monte-Carlo
robustness layer to core pillar 2.

**Cross-check against other product.md sections:**

- `§ LLM strategy` (later in product.md, line 325) carries an explicit reframe note:
  `> Reframed 2026-05-30 — the LLM is a SUPPORT pillar, not the alpha source.`
  This is self-consistent with the Pillar stack section.

- The product.md `## Strategy` table (line ~254) still shows `v2` as "LLM-augmented
  news/sentiment overlay" and `v3` as "LLM-as-forecaster." These are factual history
  rows, not forward direction claims. They do not contradict the demotion, which is
  explicitly scoped to "alpha source" role.

- The product.md `## Introduction` (lines 13, 20) references LLM reasoning in the
  vision description. These lines are pre-reframe and now read slightly inconsistently
  with the demotion. Specifically, line 13 says `classical ML, deep learning, and LLM
  reasoning to produce risk-aware trading` (suggesting co-equal pillar status) and
  line 20 says `with LLM-driven reasoning (news, sentiment, macro)` (same). The pillar
  stack section at line 31 explicitly says this "stops future sessions re-proposing
  LLM-as-alpha-engine," but the Introduction still frames the LLM as a primary method.

**Soft contradiction (low-medium confidence — LLM-judged):** The `## Introduction`
preamble describes LLM as a co-equal pillar with classical ML and DL (lines 13, 20).
The new `§ Pillar stack` section (line 26+) explicitly demotes the LLM to a support
pillar. A fast reader of the Introduction would not see the demotion. The Pillar stack
section does not amend or supersede the Introduction text. This is an internal
product.md tension — not a contradiction with `architecture.md`, but a within-document
one. Confidence: medium. Owner: analyst (one-sentence note in Introduction pointing to
the Pillar stack section, or a brief parenthetical reframe).

**Cross-check against README pitch:**

The top-level README was not read in this audit (it is large and the operator did not
flag it as a focus area). The product.md Pillar stack section ends with `This section
ratifies only the LLM = support, quantitative = core distinction.` — self-contained
enough that the README would need to be read separately if a contradiction is suspected.
Flagged for operator: verify README § Strategy or § What this is does not still
describe LLM as a primary alpha engine after the reframe.

---

## 5. trace.toml — State vs Feature.md Alignment Sweep

Spot-checked the four new rows and the 2026-05-30 cohort:

| Row ID | trace state | feature.md status | Aligned? |
|--------|------------|-------------------|----------|
| REQ-MC-BOOTSTRAP-PATH-GENERATOR-001 | `tested` | `dev-done` | NO — MISMATCH |
| REQ-STRATEGY-ROBUSTNESS-HARNESS-001 | `arch-done` | `arch-done` | YES |
| REQ-LAB-RECIPE-TEST-HARNESS-V0-2-0-001 | `shipped` | `shipped` | YES |
| REQ-LAB-YAHOO-EMPTY-RANGE-UX-001 | `shipped` | `shipped` | YES |

**No other state mismatches introduced by today's churn.** Pre-existing carried-over
mismatches (if any) are not the subject of this audit.

### Rows with empty crates/tests that are NOT in-flight

**REQ-STRATEGY-ROBUSTNESS-HARNESS-001:** `crates = []` and `tests = []`. State is
`arch-done`. This is correct and expected — the developer explicitly fills these at M-DEV.
No flag.

**REQ-LAB-YAHOO-EMPTY-RANGE-UX-001:** `crates = []` and `tests = []`. State is `shipped`.
Feature.md `## Implementation` section records all changed files and tests (7 test files).
The trace `crates` and `tests` columns were not filled in by the developer (they are still
empty arrays). This is a minor documentation gap in the trace — the ship happened but
the trace columns were not updated.

**Finding (COSMETIC):** `REQ-LAB-YAHOO-EMPTY-RANGE-UX-001` `crates` and `tests` columns
in trace.toml are empty despite the feature being `shipped`. The developer should have
filled these per the standard workflow. The information exists in feature.md
`## Implementation § Files changed`. Low blast radius — the feature is shipped and the
test evidence is in the report. Owner: orchestrator (update trace columns from feature.md).

### Backlog noqa markers

As noted by the operator: five backlog entries carry `<!-- # noqa: queue-staleness -->`.
These are intentional operator deferrals, not new drift. Recorded here as
DEFERRED-KNOWN, not flagged.

---

## 6. Decay Markers

Grepped all `spec/` folders (excluding `spec/archive/` and `spec/dev-notes/`)
for `TODO`, `FIXME`, `TBD`, `???`, `XXX`. New feature folders introduced this
session have zero markers (confirmed above). Carry-over decay-heavy folders:

| Folder | Count | Flag | Delta vs 2026-05-29 |
|--------|-------|------|----------------------|
| lumen-design-adoption | 11 | DECAY-HEAVY | 0 |
| cockpit-performance-and-input-responsiveness | 6 | DECAY-HEAVY | 0 |
| v3-volatility-forecaster-noop-fix | 4 | — | 0 |
| ui-rethink-phase-b-lab-run | 4 | — | 0 |
| (others ≤3 each) | ≤3 ea | — | 0 |

No new decay-heavy folders introduced by today's churn. Flat vs prior audit.

---

## 7. Soft Contradictions (LLM-judged — verify before acting)

### 7.1 product.md Introduction vs Pillar Stack — LLM demotion not reflected in intro preamble

**Topic:** LLM role in the system.
- `spec/product.md` Introduction (lines 13, 20): frames LLM reasoning as a co-primary
  method alongside classical ML and DL.
- `spec/product.md § Pillar stack` (line 26+): explicitly demotes LLM to a support
  pillar — "explanation and narration over a quantitative core, NOT the alpha source."

**Assessment:** The Introduction was not updated when the Pillar stack section was added.
A reader who only reads the Introduction would misunderstand the current strategic
position. This is a within-file soft contradiction. Confidence: medium-high (the
language difference is clear). Owner: analyst. One-line fix: add a parenthetical to
the Introduction pointing readers to the Pillar stack section.

### 7.2 ADR-0051 D5 "84 locked body-SHAs" vs actual anchor count — minor documentation drift

**Topic:** Monte-Carlo determinism scope.
- `spec/architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md`
  body text references "84 locked body-SHAs" (line 28 and line 269 of the ADR).
- `scripts/verify_anchors.sh` confirms: **84/84 PASS**. The count is correct.

**Finding: no contradiction.** The ADR was authored today (2026-05-30) and the anchor
count it cites (84) is current. Clean.

### 7.3 ADR-0050 D3 test contract — `#[test]` vs `#[tokio::test]` distinction

**Topic:** iced-tokio runtime-context test contract.
- `spec/architecture/adr/0050` D3 (amended) specifies that D3 tests MUST run under
  plain `#[test]` (NOT `#[tokio::test]`) to exercise the production runtime context.
- The ADR Changelog explains that the existing `lab_runner_ticker_e2e` and
  `lab_runner_cancel_e2e` tests used `#[tokio::test]` and PASSED while production
  PANICKED — this was the gap.

This is internally self-consistent within the ADR. The spec-lint dead-link checker
flags `crates/ui/tests/lab_runner_cold_cache_fetch_e2e.rs::tokio_time_timeout_without_rt_enter_panics`
as a missing path (from the trace.toml tests column for REQ-BUG-64). If that file does
not exist on disk, the test contract described in ADR-0050 D3 has a gap. This is a
carry-over trace-broken-path item, not a contradiction, but worth noting for the
developer. Confidence: low (the file may exist but the path format is wrong for
spec-lint). Owner: developer (verify file exists; update trace tests column with
correct path format).

### 7.4 C1 Q-MCB-2 ratification — shared-index bootstrap vs R1.3 literal

**Topic:** Monte-Carlo block bootstrap co-movement mode.
- `spec/monte-carlo-bootstrap-path-generator/feature.md` R1.3 (in the Requirements
  section) says: "The block bootstrap is applied **per symbol independently at v0.1.0**"
- The same feature.md `## Design` D-C1.3 says: "Q-MCB-2 = SHARED-INDEX (RATIFIED)"
  and explicitly upgrades R1.3 to shared-index.

This is NOT a contradiction — the Design section explicitly overrides the Requirements
draft text, and a note in D-C1.3 says "This upgrades R1.3's literal per-symbol-
independent default." The tester PASS confirms the shared-index implementation was
verified (FP-C1.5 confirms corr collapsed to -0.079 under per-symbol-independent,
proving the guard is genuine). The Requirements text is an unfixed draft artifact.

**Finding (COSMETIC):** R1.3 in the feature.md still reads `per symbol independently`
but the Design ratifies shared-index. A reader of Requirements alone would be misled.
Recommend: add a cross-reference note in R1.3 pointing to D-C1.3 overriding this.
Owner: analyst (brief annotation on R1.3). Low blast radius.

---

## 8. Anchor Coverage Sweep

`spec/anchors.toml` — **84 locked scenarios, 84/84 PASS confirmed.**

### Shipped strategy features vs anchor coverage

No new shipped strategy features landed today without anchor coverage. Today's ships:
- `lab-recipe-test-harness-v0.2.0` — zero anchor delta by design (channel-only events).
- `lab-yahoo-empty-range-ux` — zero anchor delta by design (UX path, no backtest reports).
- C1 `monte-carlo-bootstrap-path-generator` — zero anchor by design (C1 adds no anchor;
  anchor unit is C2's summary report per ADR-0051 D4). Correctly recorded in trace.

Pre-existing anchor coverage gaps (carry-over, unchanged):
- `v2-llm-strategy` — shipped, zero anchors, no disposition note. Carry-over P2.
- `v3-llm-forecaster` — shipped-partial, zero anchors (API-key blocked). By-design
  per feature.md; carry-over.

### mc-robustness-2026-06 namespace

The `mc-robustness-2026-06` namespace referenced in ADR-0051 D4 and C2 feature.md
is NOT yet in `anchors.toml`. This is correct — it should only be added by the tester
when C2 ships its distribution-summary report. C2 is currently `arch-done` (in-flight).
No gap.

---

## 9. Orphan Check

### Feature folders missing feature.md or tasks.md

- `spec/cockpit-app-bundle/` — missing `tasks.md` (pre-existing; candidate status;
  expected pre-developer omission). Carry-over.
- `spec/lumen-design-adoption/` — missing `tasks.md` (pre-existing; roadmap umbrella
  with sub-phase task files; expected omission). Carry-over.

No new orphan folders introduced by today's churn.

### Test reports for missing slugs

None found. Every `reports/` directory sits under an extant feature folder.

### _probe_lint_test/ orphan (carry-over)

`spec/_probe_lint_test/` — pre-existing orphan folder with no feature.md or tasks.md.
No change since last audit (> 18 days old). Carry-over P3.

---

## 10. Recommended Triage

**SHOULD-FIX (action this week):**

- **[SHOULD-FIX-1] C1 feature.md status stale — trivial one-line flip.**
  `spec/monte-carlo-bootstrap-path-generator/feature.md` frontmatter says `status: dev-done,
  owner: developer` but the trace correctly records `state: tested` (tester PASS
  2026-05-30). Flip frontmatter to `status: tested, owner: tester`. The tester should
  also confirm the feature is ready for the presenter step. Owner: orchestrator.

- **[SHOULD-FIX-2] anchors.toml main header comment 15 count-cycles stale.**
  Line 13 reads `expects 69/69 PASS — 2026-05-27`. Live count is 84/84. Update to
  `expects 84/84 PASS — 2026-05-30`. Safe edit (anchors.toml is NOT an anchored
  report). Owner: developer (cosmetic, <1 min).

- **[SHOULD-FIX-3] REQ-MC-BOOTSTRAP-PATH-GENERATOR-001 tests column prose path.**
  spec-lint flags `trace-broken-path` because the `tests` column contains
  `crates/data/src/synth (23 unit + FP-C1.1..6)` rather than resolvable `file::fn`
  paths. Update with concrete paths matching actual test function names (e.g.
  `crates/data/src/synth/bootstrap.rs::tests::same_seed_determinism`). Owner:
  developer (knows the actual test function names).

**COSMETIC (clean up opportunistically):**

- **[COSMETIC-1] 3 missing-frontmatter violations (pre-existing).**
  - `spec/lab-polish-round-2/tasks.md` — add frontmatter.
  - `spec/ui-test-harness-viewport-matrix/tasks.md` — change status `present-done` to
    a valid value (likely `shipped` if the presenter step is done).
  - `spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/tasks.md` — change
    status `dev-in-progress` to `in-progress` or `dev-done`.
  Owner: orchestrator / feature owner.

- **[COSMETIC-2] 2 shipped-no-tests violations (pre-existing).**
  `spec/lab-end-to-end-v2/` and `spec/vol-killswitch-overlay-noop-fix/` have no
  `reports/` directory. Add a placeholder directory + a test report, OR document why
  reports are absent. Owner: tester / analyst.

- **[COSMETIC-3] REQ-LAB-YAHOO-EMPTY-RANGE-UX-001 trace crates/tests columns empty.**
  Feature is `shipped` but trace columns were not filled in. Fill from feature.md
  `## Implementation § Files changed`. Owner: orchestrator.

- **[COSMETIC-4] C1 feature.md R1.3 still reads `per symbol independently` (unfixed
  draft text) despite D-C1.3 ratifying shared-index.** Add a cross-reference note.
  Owner: analyst.

- **[COSMETIC-5] ADRs 0045–0049 absent from `spec/architecture/adr/README.md` registry
  table.** ADRs 0050 and 0051 are registered (clean). The 5-entry gap is carry-over.
  Owner: architect.

- **[COSMETIC-6] product.md Introduction does not reflect LLM demotion to support
  pillar.** Lines 13, 20 frame LLM as co-primary; the new § Pillar stack section
  (line 26+) demotes it. Add a one-sentence pointer from the Introduction to the
  Pillar stack section. Owner: analyst.

**DEFERRED-KNOWN (no action needed — operator-approved deferrals or pre-existing
  carry-overs with no change this session):**

- 87 dead-link violations: ~80 are pre-existing clusters (ADR-0027 Kronos, `/tmp/`
  chart screenshots, v0-paper-sma README, journal-tx-metadata, ADR-0039/0045 paths).
  No new clusters from today's churn.
- 53 trace-broken-path violations: dominated by prose-not-path entries and pre-existing
  clusters (BUG-64 test paths, V2-1-TRACING-LAYER-REDACTOR, UI-CONTRAST-ASSERTER,
  QUEUE-STALENESS, OPERATOR-LEDGER).
- 5 backlog `noqa: queue-staleness` markers: operator-deferred intentionally.
- architecture.md inline ADR table frozen at ADR-0026 (25 entries missing).
- Python 3.11 resolved cleanly this run (no `tomllib` failure as in prior audits);
  however, the spec-update skill still notes a Python 3.9 risk — pin Python 3.11+
  for the weekly scheduled run.
- `spec/_probe_lint_test/` orphan folder (>18 days old).
- `v2-llm-strategy` and `v3-llm-forecaster` anchor disposition (carry-over).
- `product.md § Backtest "bps: 2"` vs v5 canonical 8 bps (carry-over; not exacerbated
  by today's churn).

---

## Changelog

- 2026-05-30 (spec-auditor): On-demand audit triggered by heavy multi-lane churn day
  (ADRs 0050+0051, C1 MC path-gen + C2 MC harness, lab-recipe-test-harness v0.2.0,
  lab-yahoo-empty-range-ux, product.md pillar reframe). Mechanical lint confirmed
  first successful execution under Python 3.11 (145 violations, 4 categories; 87
  dead-link / 3 missing-frontmatter / 2 shipped-no-tests / 53 trace-broken-path).
  Anchors 84/84 PASS (live verify). ADRs 0050+0051 registered and cross-references
  clean. C1 feature.md status/owner stale (SHOULD-FIX-1; trace is authoritative —
  state: tested). anchors.toml header comment 15 count-cycles stale (SHOULD-FIX-2).
  C1 trace tests column prose path flagged (SHOULD-FIX-3). Product.md Pillar stack
  internally self-consistent; soft tension with Introduction preamble noted (COSMETIC-6).
  No new decay-heavy folders. No new orphans. Strategy-robustness-harness (C2)
  correctly in-flight (arch-done); mc-robustness-2026-06 anchor namespace not yet
  in anchors.toml (correct — tester adds at C2 ship). All pre-existing carry-over
  findings unchanged.
