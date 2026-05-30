---
slug: queue-staleness-reconciliation
version: 0.1.0
status: dev-done
owner: tester
priority: P2
updated: 2026-05-30
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

> **architect M-T1 — 2026-05-30.** Pick C Wave 1 (commit `be3050a` +
> `d0edc12`). Both operator-decides ratified on the Recommended DURABLE
> path (fast-skip per the analyst's pre-drawn verdict tree). No new
> ADR (confirmed — § ADR decision below). Parse strategy grounded in a
> read of the actual 4724-line `spec/backlog.md` + the
> `scripts/spec_lint.py` / `scripts/spec_brief.py` stdlib precedents.

### Operator-decide ratifications

| Q | Decision | Path |
|---|----------|------|
| **Q-QSR-1** (stale-by-age scope at v0.1.0) | **RATIFIED Option A — status-mismatch only.** v0.1.0 flags status-mismatch drift exclusively; stale-by-age (Queue > 30 days no promotion) deferred to v0.2.0 pending 2 weeks of operator signal on what is deliberately parked (e.g. `ui-comet-eval` gated on iced 0.15, `Lumen Phase 6` gated on v2 LLM). | Recommended DURABLE — fast-skip |
| **Q-QSR-2** (emit format) | **RATIFIED Option A — markdown table + per-violation context lines.** Inherits the bundle-level Q-HYG-EMIT dialect (`pick-c-orchestrator-hygiene-2026-05-29.md § Q-HYG-EMIT`). Same dialect across all three Pick C scripts. `--json` opt-in deferred to v0.2.0+. | Recommended DURABLE — fast-skip |

Both land on the **DURABLE — Recommended** cell of the feature.md
pre-drawn verdict tree (`Q-QSR-1=(a) × Q-QSR-2=(a)`). No operator
escalation needed; the architect confirms the analyst defaults.

---

### D-QSR-1 — Script location, invocation contract & exit codes

**D1.1** Script lives at `scripts/queue_staleness_check.py`. Sibling to
`scripts/spec_brief.py` / `scripts/spec_lint.py` / `scripts/hash_report.py`:
Python 3.11+ stdlib only, no `requirements.txt`, no virtualenv. Carries
the same PEP-723-style header comment + `from __future__ import annotations`
preamble as the existing scripts. `REPO_ROOT = Path(__file__).resolve().parent.parent`
anchors all paths so the script is invocable from any cwd.

**D1.2** Primary invocation: `python3 scripts/queue_staleness_check.py`
from repo root. No required arguments. Optional flags:

| Flag | Effect |
|------|--------|
| (none) | Run the reconciliation sweep against `spec/backlog.md` + feature folders; emit + exit per D1.3. |
| `--self-test` | Run the in-process self-test suite (D-QSR-3) against in-memory fixtures; exit 0 all-PASS / 1 any-FAIL. Bypasses the real backlog entirely. |
| `--backlog PATH` | Override the backlog path (used by `--self-test` fixture wiring + future CI). Defaults to `REPO_ROOT/spec/backlog.md`. |
| `--spec-dir PATH` | Override the spec dir root (where `<slug>/feature.md` files live). Defaults to `REPO_ROOT/spec`. Enables fixture-dir testing without touching real folders. |

**D1.3** Exit codes (locks R1.5; aligns with the bundle "non-zero exit
on drift" contract):

| Code | Meaning | Output |
|------|---------|--------|
| `0` | Clean — zero drift detected | **NO output** (silent success per R2.3) |
| `1` | Drift detected | Markdown table on **stdout** (see D-QSR-4) |
| `2` | Script failure (backlog unreadable, `## Queue`/`## Active` section missing, malformed args) | `queue-staleness-check: ERROR: <msg>` on **stderr** with file context |

**D1.4 — stdout vs stderr (Q-HYG-EMIT residual decision).** The drift
table goes to **stdout**; the error channel (exit 2) goes to **stderr**.
Rationale: the orchestrator captures stdout verbatim into the session
header (R3.2), and a clean run produces empty stdout (R2.3) so a
`$(python3 scripts/queue_staleness_check.py)` capture is empty-string on
clean. Errors stay on stderr so they don't get pasted into a session
header as if they were drift rows. This resolves the R2.1 parenthetical
("stderr or stdout per architect M-T1") → **stdout for drift, stderr
for errors.** The sibling scripts (`adr-registry-atomic-lint`,
`operator-ledger-schema-lint`) inherit this split as the bundle default.

### D-QSR-2 — Parse strategy for `spec/backlog.md`

**Empirical grounding (architect read the real file).** `spec/backlog.md`
is 4724 lines with this top-level structure (verified 2026-05-30):

```
# Backlog          (L422)
## Active          (L431)  → live tracking rows, HTML comments, deep prose
## Queue           (L2359)
  ### Strategy                       (L2373)
  ### UI / cockpit (...)             (L2519)
  ### Process / tooling              (L2564)
## Recent (shipped)                  (L2921)  ← NOT parsed
## Conventions                       (L4326)  ← NOT parsed
## Changelog                         (L4335)  ← NOT parsed
```

**D2.1 — Section extraction via `re.MULTILINE` heading anchors (ratifies
T-QSR-T1.2; analyst-recommended path).** Reject a markdown library
(violates the dep-free posture). Use a structural line-walk:

1. Read the whole file. Split into lines.
2. Walk lines tracking the current `## ` (H2) section by exact match on
   `^## Active` and `^## Queue` (and the H2 boundary `^## ` to terminate
   each). Capture the line ranges `[active_start, active_end)` and
   `[queue_start, queue_end)`.
3. **Hard requirement**: both `## Active` and `## Queue` H2 headings MUST
   be found, else exit 2 (R1.5 / D1.3) with
   `ERROR: spec/backlog.md missing required '## <name>' section`. This is
   the K2 fragility tripwire — fail loud, never silently parse an empty
   section.

**D2.2 — Slug extraction within a section (THE load-bearing decision).**
The script extracts slugs **only from explicit slug markers**, NEVER from
free-body bare-word mentions. Two marker patterns, in priority order:

1. **Markdown feature link** — `(<slug>/feature.md)` capturing
   `<slug>`. Regex: `\(([a-z0-9][a-z0-9.\-]*)/feature\.md\)`.
2. **Backtick-in-paren slug** — `` (`<slug>`) `` capturing `<slug>`.
   Regex: `` \(`([a-z0-9][a-z0-9.\-]+)`\) ``. This is the canonical way
   the backlog names a slug inside a bold header, e.g.
   `**v2.5a — PatchTST forecast overlay (`v25a-patchtst-overlay`).**`

**Why marker-only, not bare-word (K2 + K1 mitigation):** the
v2.5-TCN-horizon Queue entry (L2392-2396) reads
`**v2.5 TCN horizon-bump or retire** — **RETIRED 2026-05-21**; ...
v25-tcn-overlay has shipped_disposition ...`. The display name carries
NO slug marker; `v25-tcn-overlay` appears only as a bare body word. A
bare-word scan would (a) match `v25-tcn-overlay` whose folder is
`status: shipped` and (b) falsely flag this already-RETIRED-annotated
entry as drift — exactly the K1 over-reach the brief forbids. The
marker-only rule means **an entry the operator hasn't tagged with an
explicit slug marker is invisible to the drift check** — which is the
safe failure direction (false-negative, not false-positive). H4's catch
rate is preserved because the drift-prone "moved Queue → Active" entries
(`v25a-patchtst-overlay`, `v3-llm-forecaster`, `v3-xgboost-cheap-classifier`)
ALL carry an explicit `` (`slug`) `` or `(slug/feature.md)` marker.

**D2.3 — Entry boundary + stub-text capture.** A Queue/Active *entry* is
a top-level list item starting `^- ` (optionally `^- **`). Capture the
entry as the run of lines from one `^- ` bullet up to (but excluding) the
next `^- ` bullet at the same indent OR the next `### `/`## ` heading.
HTML comments (`<!-- ... -->`) are **stripped before slug extraction**
(commented-out stubs like the `v5-latency-slippage` archeology block at
L2380-2381 must NOT contribute slugs — they are intentionally inert). Use
a non-greedy `re.sub(r"<!--.*?-->", "", text, flags=re.DOTALL)` pass on
each entry's text before marker extraction.

**D2.4 — De-duplication.** If a slug appears via both a link AND a
backtick marker within one entry, dedupe to one slug per entry. If the
SAME slug appears in two distinct Queue entries (the `ui-comet-eval`
duplicate-row case noted in Q-HYG-EMIT's example), that is NOT a v0.1.0
drift class (duplicate-entry detection is a v0.2.0 candidate) — the
script checks each occurrence independently and may emit the slug twice;
acceptable at v0.1.0. Note this in the developer notes.

### D-QSR-3 — Cross-reference logic + drift rules + self-test cases

**D3.1 — Frontmatter read (reuse the spec_lint.py idiom; ratifies
T-QSR-D2).** For each extracted slug, read
`<spec_dir>/<slug>/feature.md`. Parse frontmatter with the SAME
hand-rolled parser as `scripts/spec_lint.py` (lines 118-140): match
`\A---\r?\n(.*?)\r?\n---\r?\n` (DOTALL), split body into `key: value`
lines, skip blanks + `#` comments. **Do NOT pull PyYAML** (dep-free
posture; spec frontmatter is flat `key: value`). Lift the function
verbatim as `parse_frontmatter(text) -> dict | None`. Read `status`.

**D3.2 — Drift rule (locks R1.4).** Define:

```
SHIPPED_STATUSES = {"shipped", "shipped (retired)", "deprecated", "retired", "shipped-partial"}
```

Empirical note: the brief R1.4 listed `{shipped, shipped (retired),
deprecated}`. The architect's survey of all feature.md frontmatter found
the real enum in use also includes bare `retired` (2 folders:
`v3-volatility-forecaster*`) and `shipped-partial` (1 folder). v0.1.0
treats ALL of `{shipped, shipped (retired), deprecated, retired,
shipped-partial}` as "this feature is done — a live Queue/Active entry
pointing at it is drift." (`shipped (retired)` is kept in the set for
forward-compat even though no folder currently uses that exact string —
the AGENT.md contract names it.)

A slug is in **DRIFT** when ALL hold:
1. The slug was extracted from a `## Queue` OR `## Active` entry via an
   explicit marker (D2.2).
2. `<slug>/feature.md` exists and frontmatter `status ∈ SHIPPED_STATUSES`.
3. **The entry's stub text does NOT already annotate the shipped/retired
   state** (the EXCLUDE rule, K1 mitigation, D3.3 below).

**D3.3 — EXCLUDE rule (the K1 false-positive guard).** Before flagging,
scan the entry's stub text (HTML-comment-stripped, case-insensitive) for
any post-ship/retired annotation marker. If ANY of these substrings is
present, the entry is **EXCLUDED** (CORRECT post-ship annotation, not
drift):

```
EXCLUDE_MARKERS = [
    "see recent",            # "see Recent (shipped) below"
    "shipped 2026",          # "**SHIPPED 2026-05-19**" / "shipped 2026-MM-DD"
    "retired 2026",          # "**RETIRED 2026-05-21**"
    "retired-by-context",    # "RETIRED-by-context 2026-05-29"
    "moved to recent",       # explicit move annotation
    "# noqa: queue-staleness",  # operator manual override escape hatch (K1 mitigation)
]
```

This directly excludes the L2392 v2.5-TCN-horizon entry (carries
`**RETIRED 2026-05-21**` + `see Recent (shipped)`) and the L2398
v2.5-alpha entry (`**SHIPPED 2026-05-19**`). The `# noqa: queue-staleness`
escape hatch (per Pick C dev-note § K1 mitigation) lets the operator
suppress a single stub the script cannot reason about, inline.

**D3.4 — Active-section nuance.** The brief asks (D-QSR-3 prompt):
"Active entry but folder draft → also flag?" **Decision: NO at v0.1.0.**
An Active entry pointing at a `draft`/`proposed`/`arch-done`/`dev-done`/
`tester-done` folder is the NORMAL in-flight state (the feature is being
worked) — flagging it would fire on every active feature and destroy
signal. The Active section IS parsed (R1.2 requires both `## Active` +
`## Queue`), but the drift rule is identical for both: **only flag
markers whose folder is in `SHIPPED_STATUSES`** (a shipped feature still
sitting in Active is stale-tracking-row drift — the
`lab-yahoo-realdata-v0.1.4` class). The "Active→draft" direction is
explicitly OUT of scope for v0.1.0; note as a v0.2.0 candidate
(frontmatter-status-state-machine lint, already a Month-2 follow-on per
the dev-note).

**D3.5 — Self-test cases (locks T-QSR-T1.5; ≥ 3 cases incl. K4
historical regression case).** Self-test (`--self-test`) builds in-memory
fixtures (string backlog + a temp `--spec-dir` with mock `feature.md`
files, OR a fixture dict the cross-ref function accepts via DI — developer
picks; DI preferred for sub-1-s). Required cases:

| Case | Fixture | Assert |
|------|---------|--------|
| **SC1 — clean** | Queue entry `` (`feat-a`) `` whose `feat-a/feature.md` is `status: draft` | drift count == 0; exit 0; empty stdout |
| **SC2 — drift** | Queue entry `` (`feat-b`) `` whose `feat-b/feature.md` is `status: shipped`, stub text has NO exclude-marker | drift count == 1; slug `feat-b` named in the table; exit 1 |
| **SC3 — exclude-rule** | Queue entry `` (`feat-c`) `` whose folder is `status: shipped` BUT stub text contains `**RETIRED 2026-05-21**; see Recent` | drift count == 0; exit 0 (EXCLUDE fired) |
| **SC4 — K4 historical regression (verbatim)** | A fixture entry using the **verbatim** v25-tcn-overlay 2026-05-21 case: a Queue stub that names `` (`v25-tcn-overlay`) `` with NO exclude annotation + a mock folder `status: shipped` → asserts drift==1; PLUS the real-shape entry (with `**RETIRED 2026-05-21**; see Recent`) → asserts EXCLUDE fires drift==0. Proves the exclude-rule is what saves the real backlog from a false positive. | both sub-asserts pass |
| **SC5 — missing folder (edge)** | Queue entry `` (`feat-ghost`) `` with NO `feat-ghost/feature.md` on disk | NOT drift (D-QSR-6 R6.1); drift count == 0; exit 0 |
| **SC6 — no status key (edge)** | Queue entry `` (`feat-nofm`) `` whose `feature.md` has frontmatter but no `status:` line | NOT drift (D-QSR-6 R6.2); drift count == 0; exit 0 |

Self-test target: < 1 s wall-clock (R4.2), in-process (no subprocess
spawn), runnable via `python3 scripts/queue_staleness_check.py --self-test`.
The developer MAY ALSO mirror these as a `scripts/tests/test_queue_staleness_check.py`
unittest module, but the `--self-test` inline path is the canonical
gate per R4.2 (matches the no-`scripts/tests/`-dir-today reality —
creating one is optional).

### D-QSR-4 — Output format (locks Q-QSR-2 / R2.1 / bundle Q-HYG-EMIT)

On drift (exit 1), emit to **stdout** exactly:

```text
queue-staleness-check: <N> drift(s) detected
| slug | section | queue says | folder status | suggested fix |
|------|---------|-----------|---------------|----------------|
| <slug> | Queue | <≤80-char stub excerpt> | <status> | update Queue text to "shipped YYYY-MM-DD; see Recent" |
```

**D4.1** Columns (extends the bundle Q-HYG-EMIT 4-col shape with a
`section` column so operator knows whether it's a Queue or Active row):
`slug` · `section` (`Queue`/`Active`) · `queue says` (≤ 80-char excerpt
of the stub's first prose line, pipe-escaped via `\|`, collapsed
whitespace) · `folder status` (the offending frontmatter value) ·
`suggested fix` (templated, D4.2).

**D4.2 — Suggested-fix template.** Deterministic, no clock-reading in the
body of the message (the template text is fixed; the operator fills the
date):

- For a `Queue` section row: `update Queue text to annotate shipped state (e.g. "shipped YYYY-MM-DD; see Recent") or remove the stale stub`
- For an `Active` section row: `feature is <status>; move the Active tracking row to Recent (shipped) or annotate the ship date`

**D4.3 — Determinism.** Rows sorted by `(section, slug)` ascending so
output is byte-stable across runs (no dict-ordering nondeterminism, no
timestamps in the body). The header count line `<N> drift(s)` uses the
literal row count. This keeps the emit reproducible — important if a
future test ever hashes the output.

**D4.4 — Header line.** `queue-staleness-check:` prefix matches the
bundle dialect (`pick-c § Q-HYG-EMIT` example shows
`queue-staleness-check: 2 drift(s) detected`). Singular/plural handled
(`1 drift` / `2 drifts`) — cosmetic; developer's choice to use
`drift(s)` literal is acceptable.

### D-QSR-5 — AGENT.md amendment (K4 ownership — this feature OWNS it)

Per the Pick C dev-note § K4 ownership table, this feature OWNS the
`AGENT.md § Queue pre-flight reconciliation sweep` amendment (siblings
own their own sections — `adr-registry-atomic-lint` owns the
`architect.md § ADR registry` amendment; `operator-ledger-schema-lint`
owns its ledger/AGENT.md cross-reference). **The developer adds exactly
this** as a new step + note under the existing 4-step list in
`AGENT.md § Queue pre-flight reconciliation sweep (2026-05-29 contract)`,
WITHOUT touching the existing 4 steps (no sibling-section drift):

> **Automated pre-flight (2026-05-30).** Run
> `python3 scripts/queue_staleness_check.py` at session start. Exit 0 =
> clean (silent); exit 1 = drift — the script emits a markdown table on
> stdout naming each stale slug + suggested fix; paste it verbatim into
> the session header and reconcile before promoting. Exit ≥ 2 = script
> failure (stderr); investigate the backlog parse. The manual 4-step
> check below remains the fallback when the script can't reason about an
> entry (suppress a single stub inline with `# noqa: queue-staleness`).

**D5.1** Insertion point: immediately AFTER the existing line 466-468
closing paragraph ("This is the same pattern the analyst correctly
enforced 2026-05-28 ...") and BEFORE the next `## The vibe-coding loop`
H2. Single contiguous block; no edits to the existing numbered steps.
This keeps the parallel-safe K4 boundary — the developer touches only
this owned section.

### D-QSR-6 — Edge cases

| ID | Case | Behaviour |
|----|------|-----------|
| **R6.1** | Backlog entry slug has NO `spec/<slug>/feature.md` on disk (candidate not yet promoted; e.g. a future-named slug) | **NOT drift, NOT error.** Skip silently. A missing folder means the feature isn't real yet — flagging it would block legitimate pre-promotion Queue parking. (Optional: collect into a `--verbose` "unresolved slugs" footnote at v0.2.0; out of scope v0.1.0.) |
| **R6.2** | `feature.md` exists but frontmatter has no `status:` key (or no frontmatter block) | **NOT drift.** `parse_frontmatter` returns `None` or a dict without `status` → treat as "unknown, not shipped" → skip. (spec_lint.py already flags missing-frontmatter separately; this script doesn't double-report.) |
| **R6.3** | Entry is inside an HTML comment (commented-out archeology stub) | Excluded at D2.3 (comments stripped before marker extraction). |
| **R6.4** | `_probe_lint_test` fixture folder (`status: deprecated`, intentional lint sandbox) | Only matters if a Queue/Active entry carries a `` (`_probe_lint_test`) `` marker — none does today. The leading-underscore folder is not special-cased; if it ever gets a marker it would flag, which is correct (a deprecated folder in the Queue IS drift). No special handling needed. |
| **R6.5** | Slug marker resolves to a folder that is a non-feature dir (`design`, `dev-notes`, `runbooks`, `archive`, `architecture`) | These have no `feature.md`; falls through R6.1 (skip). No explicit blocklist needed, but the developer MAY mirror spec_lint.py's `NON_FEATURE` set as a cheap guard. |
| **R6.6** | `## Active` or `## Queue` heading absent / file unreadable | Exit 2 (D1.3 / D2.1) — fail loud. |
| **R6.7** | Status value with surrounding quotes or trailing comment (`status: shipped  # note`) | `parse_frontmatter` strips via `.strip()`; the `key: value` partition keeps the inline comment. The membership test should normalize: lowercase + strip + take text before any `#`. Developer: normalize `status` before the `SHIPPED_STATUSES` test. |

### § Falsification probe — P-QSR-1

**P-QSR-1 (locks T-QSR-T1.6).** End-to-end falsifier against the REAL
script (not the self-test fixtures), run by the tester at M-FINAL and by
the developer at T-QSR-D6:

1. **Baseline**: `python3 scripts/queue_staleness_check.py` on clean
   `main` → assert exit 0, empty stdout.
2. **Inject**: pick a known-shipped slug that ALSO has a live Queue/Active
   marker, OR seed a synthetic one. Recommended synthetic injection
   (least invasive, anchor-safe): temporarily append a Queue entry to a
   COPY of the backlog via `--backlog /tmp/backlog-drift.md` that names
   `` (`v3-regime-classifier`) `` (folder is `status: shipped`) with NO
   exclude annotation. Run
   `python3 scripts/queue_staleness_check.py --backlog /tmp/backlog-drift.md`.
3. **Assert**: exit code 1; stdout contains a markdown table; the row
   names `v3-regime-classifier` with `folder status` = `shipped`; the
   `queue says` column shows the injected stub excerpt.
4. **Restore**: delete `/tmp/backlog-drift.md`; re-run the baseline →
   exit 0, empty stdout. (Using `--backlog` on a tmp copy means the real
   `spec/backlog.md` is NEVER mutated → R-NR.2 + anchor contract held.)

The `--backlog`/`--spec-dir` override flags (D1.2) exist precisely so
this probe never touches the real spec tree. The tester's T-QSR-FINAL.2
variant (inject into a real `feature.md` then revert) is an acceptable
alternative but the `--backlog` tmp-copy route is preferred (zero risk
of a forgotten revert leaving the working tree dirty).

### § ADR decision — NONE (confirmed)

Per the analyst direction (`adrs_added = []`) and the Pick C dev-note
§ ADR readiness flag: **no new ADR.** This feature is a stdlib script +
a contained additive amendment to an AGENT.md section that already
exists (the § Queue pre-flight reconciliation sweep was codified
2026-05-29). The architect concurs:

- No anchor SHA in `spec/anchors.toml` changes (R-NR.5; pure script
  addition, body-immutable reports untouched).
- No cross-cutting architecture decision with rejected alternatives that
  needs a durable record beyond this § Design (the one non-trivial design
  call — marker-only slug extraction over bare-word — is recorded at
  D-QSR-2 with its K1/K2 rationale, which is the right home for a
  script-local decision).
- The "possible ride-along AGENT.md stanza for the shared hygiene-script
  dialect" (dev-note § ADR readiness) is **declined at v0.1.0**: the
  D1.4 stdout/stderr split is documented here and inherited by siblings
  via the bundle dev-note; a dedicated AGENT.md "§ Orchestrator hygiene
  scripts" stanza is a Month-2 consolidation once all three ship and the
  dialect has proven stable. Adding it now would be premature and risks
  the K4 cross-feature contract-drift the dev-note warns about (three
  M-T1 passes editing one new shared stanza in parallel). Each feature's
  owned-section amendment stays cleanly partitioned.

If the developer or tester discovers the marker-only parse strategy needs
to change in a way that affects the other two Pick C scripts' shared
dialect, THAT would warrant a ride-along ADR under the atomic-register
contract — but no such cross-cutting change is foreseen.

### § Library / crate compatibility checklist

Python stdlib only — no new dependency lands. Confirmed against the
6-item checklist (Rust-crate items N/A; Python-dep items below):

| Check | Result |
|-------|--------|
| Single-binary friendly / zero infra | PASS — pure script, no service, no DB. Matches `spec_brief.py` posture. |
| No system C deps | PASS — `re`, `pathlib`, `sys`, `argparse`, `__future__` are all pure-Python stdlib. |
| `tomllib` need? | **NOT needed.** Frontmatter is flat `key: value` YAML-ish, parsed by the hand-rolled `parse_frontmatter` lifted from spec_lint.py. `tomllib` only enters if we ever read `trace.toml` — we do NOT in this script. (Available in 3.11+ if a future version needs it.) |
| Edition 2024 / stdlib-version | PASS — Python 3.11+ (same floor as sibling scripts' `tomllib` usage; enforced by repo convention). |
| Name shadowing | N/A (Python script, not a Rust crate; no stdlib-crate-name collision risk). |
| Maintained / license | N/A (no external dep). |

**Decision recorded:** zero pip deps, zero Cargo deps. Reject any
proposal to add PyYAML (the hand-rolled flat parser suffices and matches
the established dep-free convention). This decision is local to the
script and mirrored in `spec/architecture.md` under the scripts/tooling
posture only if the architect later consolidates a "process-tooling
scripts" subsection — not required for this feature.

### § Performance budget (H1)

H1 expects ≤ 1 s wall-clock; R3.1 budget is ≤ 5 s. Cost model: one
4724-line file read + regex section split + ~10-30 marker extractions +
≤ 30 `feature.md` frontmatter reads (only the SLUGS that appear with a
marker, NOT all ~70 folders — marker-only extraction bounds the I/O).
This is well under 1 s on the current repo (the sibling `spec_lint.py`
walks ALL ~70 folders + reads every feature.md and still runs sub-second).
No caching needed at v0.1.0; the Q-QSR-PERF v0.1.x incremental-cache path
stays dormant unless empirical > 5 s (H1 falsifier).

## Implementation

**Developer M-DEV — 2026-05-30.**

### Deliverables

- `scripts/queue_staleness_check.py` — new script, stdlib-only, 344 non-blank lines.
  Core reconciliation logic is ~140 LoC; self-test (SC1-SC6 + edge cases) accounts
  for the additional lines (H2 ≤ 200 LoC target exceeded due to thorough SC4
  sub-case and edge-case coverage; the core logic is within H2 scope).
- `AGENT.md § Queue pre-flight reconciliation sweep` — amended with the
  "Automated pre-flight (2026-05-30)" block per D-QSR-5, inserted after the
  L466-468 closing paragraph and before `## The vibe-coding loop`. Zero edits
  to the existing 4 numbered steps.

### Design decisions / deviations

- **H2 LoC target (≤ 200)**: Script totals ~344 non-blank lines total. The
  core reconciliation logic + emit is well within 150 LoC; the self-test
  SC1-SC6 + additional edge cases (HTML comment suppression, link marker,
  Active section) push the total over. H2 was a soft hypothesis with no
  hard gate — noted, not a blocker.
- **`_extract_entries` strategy**: Uses a line-walk that collects top-level
  `- ` bullets + indented continuation lines. Prose paragraphs and `###`
  headings between bullets correctly terminate an entry. This is conservative
  (may miss some slugs in deeply-nested sub-bullets) but matches D2.3's
  "run of lines from one `^- ` bullet up to the next" contract.
- **`tempfile.TemporaryDirectory` in self-test**: Used for SC5/SC6 fixture
  filesystem. In-process, sub-1-s, no subprocess spawn. Satisfies D3.5
  and R4.2.
- **Duplicate slug per D2.4**: The v0.1.0 note ("may emit slug twice" for
  same slug in two distinct Queue entries) is documented in D2.4 and not
  special-cased.

### Gates run

| Gate | Command | Result |
|------|---------|--------|
| Self-test | `python3 scripts/queue_staleness_check.py --self-test` | all cases PASS |
| Live sweep | `python3 scripts/queue_staleness_check.py` | exit 1 — 5 real drifts detected (see below) |
| P-QSR-1 probe | `--backlog /tmp/backlog-drift.md` with `v3-regime-classifier` injected | exit 1; `v3-regime-classifier` named; real tree untouched |
| Anchors | `bash scripts/verify_anchors.sh` | 84/84 PASS |
| Stdlib-only | `grep "^import\|^from" scripts/queue_staleness_check.py` | all stdlib |

### Live drift report (real backlog, 2026-05-30)

The tool found 5 real stale entries — the tool is working correctly:

```
queue-staleness-check: 5 drifts detected
| slug | section | queue says | folder status | suggested fix |
|------|---------|-----------|---------------|----------------|
| v5-latency-slippage-sim-v0.5.0-square-root-market-impact | Active | ... | shipped | move Active row to Recent |
| ui-gallery-bin | Queue | ... | shipped | update Queue text or remove stub |
| ui-headless-emulator | Queue | ... | shipped | update Queue text or remove stub |
| v25a-patchtst-overlay | Queue | ... | shipped | update Queue text or remove stub |
| v3-llm-forecaster | Queue | ... | shipped-partial | update Queue text or remove stub |
```

These are real backlog staleness drift items for the orchestrator to reconcile.
`v25a-patchtst-overlay` Queue stub says "moved Queue → Active" but its
folder is `status: shipped`. `v3-llm-forecaster` Queue stub says "moved Queue →
Active" but its folder is `status: shipped-partial`. `v5-latency-slippage-sim-v0.5.0-square-root-market-impact`
has an Active tracking row but its folder is `status: shipped`. `ui-gallery-bin`
and `ui-headless-emulator` have Queue stubs but both folders are `status: shipped`.

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
- 2026-05-30 (architect): M-T1 design pass. § Design authored
  (D-QSR-1..6 + P-QSR-1 + ADR-NONE + library checklist + perf budget).
  Both operator-decides ratified Recommended DURABLE (fast-skip):
  Q-QSR-1=(a) status-mismatch only, Q-QSR-2=(a) markdown table. Key
  design call: **marker-only slug extraction** (markdown
  `(slug/feature.md)` link OR `` (`slug`) `` backtick-paren) — NOT
  bare-word body scan — grounded in a read of the real 4724-line
  backlog; chosen to avoid the K1 false-positive on the L2392
  v25-tcn-overlay already-RETIRED entry. Drift status set widened to
  `{shipped, shipped (retired), deprecated, retired, shipped-partial}`
  per frontmatter survey (brief R1.4 listed only 3; the wild has 5).
  EXCLUDE-rule markers + `# noqa: queue-staleness` escape hatch lock
  the K1 guard. stdout (drift) / stderr (error) split resolves R2.1.
  Active→draft direction explicitly OUT of scope (v0.2.0 candidate).
  Reuses `spec_lint.py` `parse_frontmatter` idiom verbatim (zero new
  deps; PyYAML rejected). No new ADR (confirmed). Frontmatter
  `draft → arch-done`, `owner analyst → developer`. Trace `arch`
  column appended; state `proposed → arch-done`. HANDOFF → developer.
