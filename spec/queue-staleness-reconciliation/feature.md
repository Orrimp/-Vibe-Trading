---
slug: queue-staleness-reconciliation
version: 0.1.0
status: draft
owner: analyst
priority: P2
updated: 2026-05-29
---

# Queue-staleness reconciliation — v0.1.0

> **Pick C Wave 1 promoted feature (orchestrator hygiene compounder
> trio).** Per
> [`spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`](../dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md)
> this is one of three trio pillars (~1 dev day), biased toward
> DURABLE: a `scripts/queue_staleness_check.py` orchestrator pre-flight
> script that catches stale `spec/backlog.md` Queue entries before they
> trigger reactive cleanup audits.

## Why

Per the
[`weekly-retro-2026-05-27-to-2026-05-29 § What to fix / improve #1`](../dev-notes/weekly-retro-2026-05-27-to-2026-05-29.md#what-to-fix--improve)
finding: three audits in three weeks (2026-05-07 / 2026-05-27 /
2026-05-29) caught the same drift class — Queue stubs in
`spec/backlog.md § Queue` say "moved Queue → Active YYYY-MM-DD"
while the target feature's `spec/<slug>/feature.md` frontmatter
already reads `status: shipped` or `status: shipped (retired)`. The
**analyst-halt protocol** (2026-05-28 v2.5 TCN near-miss save) is the
last line of defense — orchestrator pre-flight should catch this
BEFORE spawning an analyst on a stale Queue entry.

The contract is already codified at
[`AGENT.md § Queue pre-flight reconciliation sweep`](../../AGENT.md#queue-pre-flight-reconciliation-sweep-2026-05-29-contract)
(2026-05-29). This brief **operationalises** the contract with a
`scripts/queue_staleness_check.py` script the orchestrator invokes at
session start. Per the
[`process-tooling-survey-2026-05-29.md § Top-5 deep-dives Rank 5`](../dev-notes/process-tooling-survey-2026-05-29.md#-top-5-deep-dives-condensed):
"`scripts/queue_staleness_check.sh` greps for 'moved Queue → Active'
stubs whose target slug has frontmatter `status: shipped`; ~30 s/run
at session start. Operator already paid reactively 3× in 3 weeks."

Three layered consequences:

- **Preventive, not reactive.** The audit cycles paid ~30-45 min each
  catching this class; the script costs sub-second per session.
- **Codifies the analyst-halt save preemptively.** Operator no longer
  needs to lean on analyst pattern-matching to catch stale Queue
  entries — script catches them BEFORE any sub-agent spawn.
- **Sets the dialect for hygiene-lint scripts.** First of the trio;
  owns the shared markdown-diff dialect contract per the bundle's
  Q-HYG-EMIT ratification. Future hygiene-lint scripts (Month-2
  candidates: presentation deck approval ledger, frontmatter status
  state machine) inherit the dialect.

Per process-tooling-survey: **MEDIUM per-cycle benefit, SMALL
investment (~1d), LOW maintenance**. The 30-s/run pre-flight cost is
inverted from the 30-45-min reactive cleanup cost on each audit
cycle.

## Requirements

### R1 — Script invocation + scope

- **R1.1** A new script `scripts/queue_staleness_check.py` exists,
  invokable as `python3 scripts/queue_staleness_check.py` from repo
  root. Sibling pattern to `scripts/spec_brief.py`,
  `scripts/spec_lint.py`, `scripts/hash_report.py` (Python 3 stdlib;
  no requirements.txt).
- **R1.2** Script reads `spec/backlog.md` and parses two top-level
  sections: `## Active` and `## Queue` (with sub-sections like
  `### Strategy`, `### UI / cockpit`, `### Process / tooling`).
- **R1.3** For each entry in `## Queue` that names a feature slug
  (extracted via regex or markdown link match — architect M-T1
  ratifies the parse shape), script greps the target
  `spec/<slug>/feature.md` frontmatter `status:` field.
- **R1.4** Script identifies a **drift** when:
  - Queue stub text says the entry is `candidate` / `proposed` /
    in-flight (active development) AND feature.md frontmatter
    `status:` is one of `{shipped, shipped (retired), deprecated}`.
  - **Excludes** stubs that ALREADY annotate the shipped state (e.g.
    "shipped 2026-MM-DD; see Recent") — these are CORRECT
    post-ship annotations per AGENT.md § Queue pre-flight § step 2.
- **R1.5** Script exits with code 0 on clean (zero drift); code 1 on
  drift detected; code ≥ 2 on script failure (broken markdown parse,
  missing file, unreadable frontmatter).

### R2 — Operator-friendly diff output

- **R2.1** On drift, script writes to stderr (or stdout per architect
  M-T1 ratification of Q-HYG-EMIT bundle default) a markdown-formatted
  diff message per bundle Q-HYG-EMIT ratification:
  ```text
  queue-staleness-check: <N> drift(s) detected
  | slug | queue says | folder status | suggested fix |
  |------|-----------|---------------|----------------|
  | <slug> | <queue stub excerpt> | <frontmatter status> | <suggested edit> |
  ```
- **R2.2** Each drift row includes:
  - The slug
  - A short excerpt (≤ 80 chars) of the Queue stub showing the stale
    state claim
  - The current frontmatter `status:` value
  - A suggested fix (templated; e.g. "update Queue text to 'shipped
    YYYY-MM-DD; see Recent'")
- **R2.3** Clean run produces ZERO output (silent success) — no
  noise on the operator's session header.
- **R2.4** Failure (exit ≥ 2) produces an error-prefixed message
  with file:line context for debugging.

### R3 — Orchestrator wire-up

- **R3.1** Orchestrator invokes the script at session start as the
  Queue pre-flight reconciliation sweep per the AGENT.md contract.
  Wall-clock budget: ≤ 5 s on the current repo (≈ 50 Queue entries,
  ≈ 70 active feature folders). H1 expects sub-1-s; if >= 5 s,
  Q-QSR-PERF triggers v0.1.x with an incremental cache.
- **R3.2** Orchestrator includes the script's drift output verbatim
  in the next session header (if any drift). Per the bundle Q-HYG-EMIT
  contract, the output is already markdown-shaped — no wrapping
  needed.
- **R3.3** AGENT.md § Queue pre-flight reconciliation sweep gets
  amended in this feature's M-T1 close to include a 1-line invocation
  example: `python3 scripts/queue_staleness_check.py` (or equivalent).
  This brief OWNS this AGENT.md amendment per bundle K4
  ownership-table.

### R4 — Self-test (script smoke)

- **R4.1** Script has a `#[cfg(test)]`-equivalent self-test in
  `scripts/tests/test_queue_staleness_check.py` (or inline
  `if __name__ == "__main__":` smoke per architect M-T1 dialect
  choice). Self-test:
  - Constructs an in-memory mock backlog + mock feature.md frontmatter
  - Asserts zero drift on a clean state
  - Asserts one drift on a state where Queue says "candidate" and
    frontmatter says "shipped"
  - Asserts exclude-rule fires on Queue text that already annotates
    shipped state
- **R4.2** Self-test runs in < 1 s; invokable via
  `python3 -m unittest scripts/tests/test_queue_staleness_check.py`
  OR `python3 scripts/queue_staleness_check.py --self-test` per
  architect M-T1.

### R-NR — Non-regression contract

- **R-NR.1** Zero edits to `spec/<slug>/feature.md` frontmatter from
  this brief — the script is READ-ONLY on feature.md files.
- **R-NR.2** Zero edits to `spec/backlog.md` from this brief — the
  script REPORTS drift; it does NOT auto-fix. (Auto-fix is a
  v0.2.0+ scope decision per Q-QSR-AUTOFIX honorable mention.)
- **R-NR.3** Zero new Cargo.toml deps — pure Python 3 stdlib.
- **R-NR.4** Zero new external Python deps — no requirements.txt
  addition.
- **R-NR.5** `bash scripts/verify_anchors.sh` → all-PASS byte-identical
  pre/post. Pure script addition; no anchor delta.
- **R-NR.6** Anchored report files under `spec/<slug>/reports/*.md`
  are NOT read by the script — only `spec/backlog.md` + each
  `spec/<slug>/feature.md` frontmatter.
- **R-NR.7** No new clippy / fmt deltas (script is Python, not Rust).

## Falsifiers (K)

- **K1 — Script over-reach blocks legitimate Queue promotions.** Per
  the bundle direction § Risk K1: the Queue-staleness script may
  flag CORRECT post-ship annotations or intentionally-deferred Queue
  entries as drift. **Mitigation**: feature.md R1.4 exclude-rule
  table; v0.1.0 ONLY flags `status: shipped | shipped (retired) |
  deprecated` against active Queue stubs (not "candidate" stubs
  whose feature has draft / proposed / in-progress status). Diff
  message includes per-row context for operator triage.
- **K2 — Markdown parsing fragility on the 4566-line backlog.**
  `spec/backlog.md` carries HTML comments, nested bullets, complex
  cross-references, and code blocks. A naive regex may match against
  these. **Mitigation**: architect M-T1 picks parse shape (lightweight
  markdown structural parse via `re.MULTILINE` with section anchors
  ≥ deep markdown library). H2 expects ≤ 50 LoC of parse code.
- **K3 — Stale-by-age false positives.** Operator may have Queue
  entries deliberately parked for months (e.g. `ui-comet-eval` gated
  on iced 0.15 release). v0.1.0 OUT-OF-SCOPE per Q-QSR-1=(a) DURABLE.
  Stale-by-age detection deferred to v0.2.0+ when operator signals
  signal-vs-noise feedback.
- **K4 — Self-test drifts from real-world ambiguity.** A self-test
  on mock data may pass while the script breaks on the real
  4566-line backlog. **Mitigation**: feature.md R4.1 self-test
  includes an inline regression case using a verbatim excerpt of a
  known historical drift case (e.g. the 2026-05-21 v25-tcn-overlay
  stale-stub case).

## Hypotheses (H)

- **H1 — Script wall-clock ≤ 1 s on current repo.** Python stdlib
  markdown parse + ~50 feature.md frontmatter reads complete in
  well under the 5-s budget. Falsifier: empirical > 5 s → route to
  Q-QSR-PERF with v0.1.x cache patch.
- **H2 — Script ≤ 200 LoC.** Section parse + slug extraction +
  frontmatter read + drift-rule + markdown-table-emit fits within
  ~150 LoC; +50 LoC for argparse / self-test. Matches the analyst's
  ~1d estimate.
- **H3 — Zero existing tests break.** Pure script addition; no
  production code, no Cargo touched.
- **H4 — Operator catches ≥ 1 drift in the first 2 weeks of
  use.** Given 3 audit cycles in 3 weeks pre-script, expected
  empirical catch rate is high. Falsifier: 0 drift in first 2 weeks
  means either (a) script is over-tight (K1 fired), or (b) the
  recent process-tooling-survey cleanup already retired all known
  drift cases — confirms preventive vs reactive value.

## Operator decisions

### Q-QSR-1 — Stale-by-age scope at v0.1.0

**Q.** Does the v0.1.0 script ALSO detect stale-by-age Queue entries
(Queue stub > 30 days old with no promotion activity) and surface
them, OR does it ONLY flag status-mismatch drift?

**(Recommended — DURABLE) Option A — status-mismatch only at v0.1.0.**
Stale-by-age is a SOFTER signal class that benefits from empirical
operator feedback on which entries are legitimately parked. v0.1.0
ships narrow on status-mismatch (the audited 3× cleanup class);
v0.2.0 adds stale-by-age once operator has 2 weeks of v0.1.0 signal
to anchor "what's deliberately parked vs what's truly stale."

**Cost.** ~0 (Option A is the natural v0.1.0 scope; stale-by-age
adds ~30 LoC + 1 operator-decide at v0.2.0).

**Rationale (DURABLE).** Per AGENT.md 2026-05-28: ship the narrow,
clearly-load-bearing case first; observe; expand with operator
data. The cheap-and-quick path (gate stale-by-age in v0.1.0) risks
K1 over-reach and flags `ui-comet-eval` / parked-for-iced-0.15
candidates as drift. Two operator approval cycles for the expanded
scope is durable framing; one cycle with K1 over-reach is brittle.

**Option B (cheap fallback — REJECTED at analyst level).** Include
stale-by-age at v0.1.0 with a 30-day default. **Rejected** per K1
risk: parked Queue entries get flagged with no clear "this is
intentional" signal from the operator. Routes back to analyst with
the parked-entry tagging shape, costing more than splitting into
v0.1.0 + v0.2.0.

**Default**: A (Recommended DURABLE).

### Q-QSR-2 — Diff output format (table vs JSON)

**Q.** Does the script emit a markdown table (per bundle Q-HYG-EMIT
default) OR JSON for tooling integration?

**(Recommended — DURABLE) Option A — markdown table per bundle
Q-HYG-EMIT.** Inherits the bundle-level Q-HYG-EMIT ratification at
[`pick-c-orchestrator-hygiene-2026-05-29.md`](../dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md).
Markdown table renders cleanly in orchestrator session headers, PRs,
chat. Same dialect across all three Pick C scripts.

**Cost.** ~0 — markdown table is the same overhead as plain prose
emit. Same as bundle Q-HYG-EMIT.

**Rationale (DURABLE).** Per the bundle direction § Q-HYG-EMIT:
operator pastes the diff into chat / PR to triage, not into a tool
to parse. JSON adds a wrapping step. Future tooling integration
gets a `--json` opt-in flag at v0.2.0+.

**Option B (cheap fallback — REJECTED at analyst level).** JSON
default with a `--markdown` flag for chat use. **Rejected** per
the bundle Q-HYG-EMIT rationale — fragments the orchestrator's
mental model.

**Default**: A (Recommended DURABLE; inherited from bundle
Q-HYG-EMIT).

## Verdict tree (pre-drawn)

| Q-QSR-1 \ Q-QSR-2 | Q-QSR-2=(a) markdown | Q-QSR-2=(b) JSON |
|---|---|---|
| **Q-QSR-1=(a) status-mismatch only** | **DURABLE — Recommended.** Narrow scope + universal-paste output; bundle-aligned. | INCONSISTENT — narrow scope but tool-format breaks bundle dialect. Operator-override only. |
| **Q-QSR-1=(b) include stale-by-age** | INCONSISTENT — wide scope risks K1 over-reach; markdown emit OK but operator feedback loop undeveloped. | REJECTED — wide scope + bundle-dialect-break: worst of both. |

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
  R1-R4 + R-NR (7 clauses) + K1-K4 + H1-H4 + Q-QSR-1/2 +
  pre-drawn 4-cell verdict tree. Both Qs bias DURABLE per AGENT.md
  2026-05-28. AGENT.md § Queue pre-flight reconciliation sweep
  amendment OWNED by this brief per bundle K4 ownership-table.
  Trace row `REQ-QUEUE-STALENESS-RECONCILIATION-001` opened
  `proposed`. HANDOFF → architect (M-T1 + bundle parallel block
  with `adr-registry-atomic-lint` + `operator-ledger-schema-lint`
  siblings).
