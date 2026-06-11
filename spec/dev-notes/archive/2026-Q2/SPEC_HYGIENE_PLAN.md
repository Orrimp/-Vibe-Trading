---
title: Spec Hygiene Improvement Plan
status: proposal
author: audit-2026-05-13
version: 0.1.0
intended-location: spec/dev-notes/2026-05-13-spec-hygiene-plan.md
---

# Spec Hygiene Improvement Plan

A concrete, sequenced plan to close the gaps identified in the 2026-05-13 audit
of the 83k-line `spec/` tree. Each item declares: **What** is built, **Why** it
exists (which audited risk it closes), **How** it plugs into the existing
analyst → architect → developer ‖ ui-designer → tester → presenter workflow,
and **Acceptance** signals so you know it's done.

The plan is sequenced so each phase unblocks the next. Skip-ahead is possible
but Phase 1 is genuinely foundational — most later work depends on the
traceability index existing.

---

## Audit recap

You already have: feature lifecycle states in frontmatter, spec versioning,
report archival, an effective regression-anchor gate, determinism discipline,
sub-agent capability boundaries. You're missing: spec linting, a traceability
index, a spec-auditor agent, reverse-direction validation, structured
handoffs, and contract-level anchors. The single biggest concrete risk is
that `spec/architecture.md` is 296 KB / 5,635 lines / 144 section
headers — too large for any agent to read in one turn.

---

## Phase 1 — Foundation (highest leverage)

### 1A. Split `architecture.md` into navigable sections

**What.** Decompose `spec/architecture.md` into a thin index file plus a
`spec/architecture/` directory of per-concern files (~500-1500 lines each).

```
spec/
├── architecture.md              # ~300 line index: TOC + invariants only
└── architecture/
    ├── 00-system-overview.md
    ├── 01-determinism-and-money.md
    ├── 02-data-pipeline.md
    ├── 03-strategies-and-composition.md
    ├── 04-execution-and-venues.md
    ├── 05-cockpit-and-ui.md
    ├── 06-llm-strategy.md
    ├── 07-regression-gate.md
    ├── 08-deployment-and-ops.md
    └── adr/
        ├── 0001-rng-chacha20.md
        ├── 0002-decimal-money-math.md
        ├── 0003-sub-agent-capability-boundary.md
        └── ...
```

**Why.** Closes the "architecture.md unreadable in one turn" risk. Lets an
agent load only the section relevant to its task. ADRs preserve decision
history that gets lost in monolithic edits.

**How.**
1. One-time migration via a dedicated session: have a sub-agent propose the
   split, you approve, then `spec-update` applies it.
2. Convert each "we adopted X because Y, see incident Z" passage in the
   current architecture.md into a numbered ADR.
3. The new `architecture.md` index keeps only: invariants table, ADR registry,
   per-section pointers. Nothing substantive in the root file.

**Acceptance.**
- No file under `spec/architecture/` exceeds 1,500 lines.
- Every section in `architecture.md` index links to a body file.
- A fresh agent can be asked "what's the rule on money math?" and load only
  `01-determinism-and-money.md` plus `adr/0002-decimal-money-math.md` (~2k
  tokens) instead of 5,635 lines.

**Effort.** 1-2 sessions; high one-time cost, permanent payoff.

---

### 1B. Machine-readable traceability index

**What.** A single TOML file: `spec/trace.toml`. Each row binds a requirement
to its downstream artifacts.

```toml
[[req]]
id          = "REQ-DETERMINISM-001"
title       = "All RNG must be seeded with ChaCha20 from a config-supplied seed"
product     = "product.md#determinism"
arch        = ["architecture/01-determinism-and-money.md#rng",
               "architecture/adr/0001-rng-chacha20.md"]
feature     = []                              # cross-cutting
crates      = ["crates/core-rng", "crates/backtest"]
tests       = ["crates/core-rng/tests/determinism.rs"]
anchors     = ["btc-2023-1m-sma-cross", "btc-2023-1m-macd-trend"]
state       = "shipped"

[[req]]
id          = "REQ-CHART-SELL-EMPHASIS-001"
title       = "Sell markers must be visually dominant over buy markers on the chart"
product     = "product.md#chart-readability"
arch        = ["architecture/05-cockpit-and-ui.md#markers"]
feature     = "chart-buy-sell-emphasis"
crates      = ["crates/cockpit-chart"]
tests       = ["crates/cockpit-chart/tests/marker_emphasis.rs"]
anchors     = []
state       = "shipped"
```

**Why.** This is what lets you answer "what does this code touch?" and
"what spec covers this module?" mechanically. Orphans (any row missing a
required field, any code path with no `req`) become detectable. Closes the
silent-drift risk.

**How.**
1. Add a `trace-update` sub-skill under `.claude/skills/spec-update/`
   (or as its own skill) that knows how to insert/edit rows atomically.
2. Make `trace.toml` ownership shared: analyst writes the `[req]` row when a
   feature enters `proposed`; architect fills `arch`; developer fills
   `crates` + `tests`; tester fills `anchors`. The state column tracks
   lifecycle.
3. Bootstrap by reverse-engineering trace rows from existing shipped features
   in one back-fill session (one row per `feature.md` is the minimum).

**Acceptance.**
- Every shipped feature has at least one `[req]` row.
- Every `[req]` row's `arch`, `crates`, `tests` paths actually exist (this is
  the spec-lint check from Phase 2).
- `grep "REQ-CHART"` finds the row instantly; ditto by crate name.

**Effort.** Schema design: 1 session. Back-fill: 1-2 sessions for ~25
features. Ongoing: ~5 minutes per new feature.

---

## Phase 2 — Detection

### 2A. `spec-lint` skill

**What.** A new skill `.claude/skills/spec-lint/SKILL.md` plus
`scripts/spec_lint.py` that returns non-zero on any of:

- Dead intra-spec link (any markdown link to `spec/**` whose target doesn't
  exist).
- `feature.md` missing required frontmatter keys: `title`, `version`,
  `status`, `owner`.
- Orphan feature folder (no `feature.md` or no `tasks.md`).
- `trace.toml` row referencing non-existent crate, test file, or anchor.
- Anchor in `anchors.toml` not referenced by any `trace.toml` row.
- Feature in `status: shipped` with no test row in `trace.toml`.
- ADR file not registered in the `architecture.md` ADR registry.

**Why.** Currently you only mechanically verify the 9 backtest-report SHAs.
This extends mechanical enforcement to spec structure itself.

**How.** Pure Python script (Decimal-style: no Rust dependency). Add a
`make spec-lint` target. Wire it into the same place `verify_anchors.sh`
runs — wherever your CI / pre-commit gate is. If you don't have one yet,
add a `scripts/precommit.sh` aggregator.

**Acceptance.**
- `spec_lint.py` exits non-zero on a deliberate injected broken cross-ref.
- Tester's report template references "spec-lint: PASS" as a required line.
- AGENT.md presenter gate updated to block on spec-lint regression.

**Effort.** 1 session.

---

### 2B. `spec-auditor` agent

**What.** A new agent `.claude/agents/spec-auditor.md` that runs weekly (via
your scheduled-tasks tool) and on demand. It produces
`spec/dev-notes/audit-YYYY-MM-DD.md` listing:

- Specs with no implementing code (orphan requirements).
- Code modules with no `trace.toml` row (orphan code).
- Features stuck in `proposed` or `in-progress` for >30 days.
- Contradictions: two specs making opposing claims about the same component
  (LLM-judged — this one is fuzzy and produces a punch-list, not a hard fail).
- Stale TODO / FIXME / "TBD" markers in spec/.
- Anchor coverage gaps (shipped feature with no anchor and no documented
  exemption).

**Why.** Spec-lint catches mechanical breakage; the auditor catches semantic
drift and decay. Together they replace "operator notices the drift later"
with proactive surfacing.

**How.** Agent is read-only — it does not edit spec files; only emits a
dev-note. Operator triages. Schedule via `mcp__scheduled-tasks__create_scheduled_task`
with cron `0 9 * * 1` (Monday 09:00).

**Acceptance.**
- Weekly dev-note appears in `spec/dev-notes/` without manual prompting.
- Last 4 audit notes show the punch list shrinking, not growing.

**Effort.** 1 session to write the agent prompt + schedule it.

---

## Phase 3 — Context delivery

### 3A. Pre-flight briefing assembly skill

**What.** A skill `.claude/skills/spec-brief/SKILL.md` invoked by the
orchestrator before each sub-agent runs. Given a feature slug (or a
`trace.toml` row ID), it produces a single ~3-5k-token briefing pack
containing:

1. The relevant `[req]` rows from `trace.toml`.
2. The named architecture section(s) referenced by those rows (full text).
3. Relevant ADRs.
4. The current `feature.md` + `tasks.md`.
5. Last tester report for the feature (if any).
6. Open questions from the most recent handoff envelope (Phase 4).
7. The non-negotiables from `CLAUDE.md` (always included verbatim).

**Why.** Solves "architecture.md too big to read" at point of use. Sub-agents
work from a curated brief instead of grepping a 5,635-line file. Pairs with
1A but is independently valuable even before the architecture split.

**How.** Python script that consumes `trace.toml` + a feature slug and writes
`/tmp/brief-<slug>.md`. AGENT.md prescribes calling `spec-brief` before
spawning any architect or developer sub-agent for a specific feature.

**Acceptance.**
- Spawning a developer for `chart-canvas-overhaul` produces a brief of
  ~3-5k tokens that contains everything that agent needs (verified by a
  one-off "could you have done your job from just this brief?" check).
- Average sub-agent input context drops measurably (track via a one-line
  metric in tester reports).

**Effort.** 1-2 sessions.

---

## Phase 4 — Handoff fidelity

### 4A. Structured handoff envelope

**What.** Every agent emits a TOML block alongside its prose report, e.g.:

```toml
[handoff]
from        = "architect"
to          = "developer"
feature     = "chart-canvas-overhaul"
trace_refs  = ["REQ-CHART-CANVAS-001", "REQ-CHART-CANVAS-002"]
verdict     = "READY"
priority    = "P1"

[inputs]
brief       = "spec-brief artifact id: brief-2026-05-13-001"
artifacts   = ["spec/chart-canvas-overhaul/architecture-2026-05-13.md"]

[outputs]
spec_files  = ["spec/chart-canvas-overhaul/feature.md"]
adrs_added  = ["adr/0014-canvas-z-ordering.md"]

[open_questions]
items = [
  "Does the marker-emphasis change require a regression anchor refresh?",
  "Should the legend re-render strategy be parameterized or hard-coded?",
]

[assumptions]
items = [
  "Iced 0.13 stable for the duration of this feature",
  "No new wgpu dependency",
]
```

**Why.** Cuts the telephone-game loss between agents. The receiving agent
reads the envelope first, prose second. Open questions and assumptions become
mechanically detectable, not buried in prose.

**How.** AGENT.md update: handoff format spec. `spec-update` skill validates
on write. Optional: a tiny `scripts/handoff_lint.py` that checks every
report has a parseable envelope.

**Acceptance.**
- Every new report after the cutoff date has a parseable envelope.
- Open questions across the last 3 reports are auto-collated into a single
  list by `scripts/open_questions.py` (trivial follow-on once envelopes exist).

**Effort.** Spec the format: half session. Adoption: rolling, as each agent
runs.

---

## Phase 5 — Drift prevention (ambitious)

### 5A. Reverse-direction validation

**What.** A periodic agent task: read selected crates and produce
`spec/dev-notes/reverse-spec-YYYY-MM-DD.md` describing what the code
*appears* to do. Diff against the corresponding `architecture/*.md` section.
Flag mismatches.

**Why.** The single failure mode you can't currently catch: code that quietly
no longer matches the design doc. Worth the cost only after Phase 1 + 1B
exist, because otherwise the diff target is too fuzzy.

**How.** Run quarterly per crate, not weekly. One sub-agent per crate;
operator triages mismatches. Mismatches resolve by either fixing the code or
updating the spec (and an ADR explaining why).

**Acceptance.** First run produces a non-trivial diff list; second run (after
fixes) is shorter. After 2-3 cycles, the diff stays small.

**Effort.** First cycle is expensive (1 session per major crate). Steady-state
~1 hour per crate per quarter.

---

### 5B. Behavioral and contract anchors

**What.** Extend `anchors.toml` beyond backtest-report SHAs:

```toml
[[behavior_anchor]]
id          = "BHV-ORDER-REJECT-NOTIONAL"
description = "Order with notional < min_notional must reject with InvalidNotional error variant"
trace_ref   = "REQ-EXEC-REJECT-001"
test        = "crates/execution/tests/order_validation.rs::reject_below_min_notional"

[[contract_anchor]]
id          = "CONTRACT-STRATEGY-TRAIT"
description = "Strategy trait signature - any breaking change requires architect approval and ADR"
file        = "crates/strategy-core/src/lib.rs"
symbol      = "trait Strategy"
sha256      = "ab12cd..."   # hash of the symbol body
```

**Why.** Closes the gap "what about silent breaking changes to internal
APIs?" Body-SHA on a trait declaration catches signature drift. Behavior
anchors map named requirements to specific tests, making test-as-spec
explicit.

**How.** Extend `verify_anchors.sh` to handle the new sections. Contract
anchors use the same body-SHA primitive; behavior anchors just verify the
named test exists and is not `#[ignore]`d.

**Acceptance.** A deliberate signature change to a contract-anchored trait
fails CI without an anchor update.

**Effort.** 1 session for schema + script; rolling adoption.

---

## Suggested sequencing

| Week | Phase | Outcome |
|------|-------|---------|
| 1 | 1A (split architecture.md), 1B schema design | architecture.md navigable; trace.toml format approved |
| 2 | 1B back-fill from shipped features | Every shipped feature has a trace row |
| 3 | 2A (spec-lint) | CI fails on broken cross-refs / missing frontmatter |
| 3 | 2B (spec-auditor agent + weekly schedule) | First auto-audit dev-note lands |
| 4 | 3A (spec-brief skill) | Sub-agents work from curated briefs |
| 5 | 4A (handoff envelope) | New reports carry structured envelopes |
| 6-7 | 5A first cycle | First reverse-spec dev-note per major crate |
| 8 | 5B (extended anchors) | Trait signatures locked |

Phases 1 and 2 give you 80% of the value. Phases 3-5 are diminishing returns
but each closes a named risk.

---

## Things deliberately not in this plan

A few seductive additions I considered and rejected:

- **A graph database / Neo4j for the traceability index.** TOML in git is
  durable, diff-friendly, agent-readable, and has zero ops cost. Don't.
- **A web UI for the traceability index.** Same reason. `grep` and `bat` are
  enough until they aren't.
- **Replacing `architecture.md` with a wiki.** You lose the SHA-anchorable,
  diffable, branchable property of plain markdown in git. The split into
  `architecture/*.md` gives you 90% of the navigation benefit with none of
  the loss.
- **A custom CLI tool for spec-update / spec-lint.** Python scripts behind
  named skills are simpler, faster to evolve, and don't require a Rust build
  to lint markdown.

---

## How to apply this plan

This file lives at the repo root as a proposal. To adopt it, use your
`spec-update` skill to move it into `spec/dev-notes/2026-05-13-spec-hygiene-plan.md`
and create a feature folder `spec/spec-hygiene-overhaul/` with a
`feature.md` and `tasks.md` derived from the phase table above.

Then the work plays through the normal analyst → architect → developer →
tester loop. Phase 1A is the natural first sprint because everything else
benefits from it landing.
