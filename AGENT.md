# AGENT.md — Orchestration & Workflow

This document is the contract for how the six specialist agents collaborate
on the Rust crypto trading agent. It is **required reading** for any Claude
session acting as the orchestrator.

> **File precedence for AI agents**: read [README.md](README.md) first
> (project orientation + status snapshot + feature groups + quickstart),
> then [CLAUDE.md](CLAUDE.md) (coding rules + non-negotiables), then this
> file (orchestration). AGENT.md is the third file in the chain.

> This project uses sub-agents and expects them to run in **parallel** whenever
> their work is independent. Sequential handoffs are only for dependent work.

## Branch & worktree policy (load-bearing)

**All work happens directly on the `main` branch of the main repo at
`/Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading`. No feature
branches. No git worktrees. No `claude/<slug>` branches.**

Rules — read these before spawning anything:

1. **Orchestrator CWD is the main repo.** Verify with `pwd` at session start.
   Never operate from `.claude/worktrees/<name>/`. If a prior session left a
   worktree behind, propose cleanup; do not silently use it.
2. **Orchestrator HEAD is `main`.** Verify with `git branch --show-current`.
   Never `git checkout -b`, never create a branch.
3. **Sub-agents do NOT commit.** They write files; the orchestrator owns
   `git add` + `git commit` + `git push origin main`. A sub-agent that ends
   its run with a commit has violated the contract.
4. **Brief sub-agents with `main` as the working branch.** When a sub-agent
   prompt mentions a working directory or branch, it is the main repo path
   and `main`. Do not pass `isolation: "worktree"` to the Agent tool.
5. **No PRs unless the operator asks.** Push directly to `origin/main`.
   `gh pr create` is reserved for explicit operator request — the
   workflow does not assume code review on a PR surface.
6. **Parallelism is preserved.** Multiple sub-agents still run in parallel
   per `## Parallelism rules` below. Parallelism is about *concurrent work
   in the same tree*, not about *parallel branches*. The orchestrator
   serializes commits at the end of each wave.

*Why:* (a) every prior worktree session ended with a fast-forward merge to
main anyway — pure ceremony for a single-operator codebase. (b) Agents
historically wrote to the wrong tree when CWD and brief paths diverged.
(c) Established 2026-05-16 after the spec-hygiene remediation pass.

## The six agents

| Agent        | Model  | File                                  | Primary role                              |
|--------------|--------|---------------------------------------|-------------------------------------------|
| analyst      | opus   | `.claude/agents/analyst.md`           | Research, requirements, critique          |
| architect    | opus   | `.claude/agents/architect.md`         | System design, crate layout, tradeoffs    |
| developer    | sonnet | `.claude/agents/developer.md`         | Rust implementation, tests alongside code |
| ui-designer  | opus   | `.claude/agents/ui-designer.md`       | iced UI: implement + consistency + human-friendliness |
| tester       | sonnet | `.claude/agents/tester.md`            | Build, test, validate, backtest, report   |
| presenter    | opus   | `.claude/agents/presenter.md`         | Operator-facing presentations + agile approval loop |

Opus for deep thinking (analyst, architect, ui-designer, presenter — communication is hard).
Sonnet for high-throughput execution (developer, tester).

## Canonical workflow

```
                  ┌─────────────┐
                  │   analyst   │  opus  — requirements, research
                  └──────┬──────┘
                         │ HANDOFF (spec/<slug>/feature.md)
                         ▼
                  ┌─────────────┐
                  │  architect  │  opus  — design, task breakdown
                  └──────┬──────┘
                         │ HANDOFF (spec/<slug>/tasks.md)
              ┌──────────┴───────────┐
              ▼                      ▼
       ┌─────────────┐        ┌──────────────┐
       │  developer  │        │ ui-designer  │  opus — UI surface
       │   sonnet    │        │              │  (parallel to dev)
       └──────┬──────┘        └──────┬───────┘
              │                      │
              └──────────┬───────────┘
                         │ HANDOFF (changed crates, commands)
                         ▼
                  ┌─────────────┐
                  │   tester    │  sonnet — validate, bench, backtest, REPORT
                  └──────┬──────┘
                         │ VERDICT or HANDOFF
                         ▼
                ┌────────────────────────┐
                │ verdict routing:       │
                │  PASS → presenter      │
                │  FAIL → developer      │
                │       or ui-designer   │
                │  REGRESSION:           │
                │   structural → architect │
                │   strategy   → analyst   │
                │   UX/visual  → ui-designer │
                └────────┬───────────────┘
                         │ on PASS
                         ▼
                  ┌─────────────┐
                  │  presenter  │  opus — distills work into a sprint-review
                  └──────┬──────┘  presentation; runs real bins, captures
                         │         screenshots, lists open decisions
                         ▼
                  ┌─────────────┐
                  │    human    │  approves / approves-with-notes / rejects
                  └─────────────┘
                         │ on rejection: feedback routes to the named agent
                         │ (analyst / architect / developer / ui-designer)
```

Feedback edges are **first-class**. The tester's report is not a terminator —
it is an input to whichever agent owns the failure mode. UI-designer and
developer run **in parallel** whenever a feature has both backend and UI
work; they synchronize only on shared types in the `core` crate. The
presenter is the human-facing terminator: nothing ships without an
operator approval recorded against a presentation.

## Parallelism rules

**Default to parallel.** The orchestrator (the main Claude session) MUST
spawn sub-agents in parallel whenever their tasks are independent. Sequential
execution is the exception, not the default — and it must be justified by a
real file-scope conflict, an explicit dependency edge in the task graph, or
an operator-decide gate that hasn't been answered yet.

When the operator hands the orchestrator a multi-item agenda (e.g. "do
items A, B, C, D, E"), the orchestrator's FIRST action is to evaluate
which items can run concurrently and present a wave plan to the operator
**before** spawning any agents. Use the **file-scope conflict matrix**
below.

### File-scope conflict matrix (orchestrator pre-spawn checklist)

Before spawning any wave of agents, for every (agent_i, agent_j) pair in
the wave answer YES/NO to each of these:

1. **Touches the same file?** If both will edit the same `crates/.../*.rs`
   or `spec/.../*.md`, they conflict. SEQUENCE them or carve out
   non-overlapping line ranges and document the carve-out in both briefs.
2. **Touches the same module's public API?** If A introduces a new
   `pub fn foo()` and B imports `bar::*`, B is going to have to rebase.
   Spawn A first, then B.
3. **Same `Cargo.toml`?** Two agents both adding deps/features/[[test]]
   entries to the same Cargo.toml conflict. SEQUENCE.
4. **Same generated artifact?** Anchored reports, insta snapshots,
   gallery snapshots — two agents both regenerating these will collide.
5. **Same operator-decide question?** If two agents need the SAME Q
   answered before they can converge, defer to the operator first;
   don't spawn both on optimistic defaults — you'll re-spawn one of them.

If every cell is NO, the pair is safe to spawn concurrently. If any cell
is YES, the conflicting agents go into sequential waves.

### Wave-based scheduling

For multi-item agendas, partition into waves:

- **Wave 1**: every item whose file-scope is independent of every other
  Wave 1 item. Spawn ALL of them in a single Agent tool-use block.
- **Wave 2**: items that conflict with at least one Wave 1 item OR
  depend on Wave 1's output. Spawn after Wave 1 lands.
- **Wave 3+**: same rule, applied recursively.

A 7-item agenda commonly maps to 1-3 waves with 3-5 agents per wave.
That's normal. Avoid the antipattern of "spawn agent 1, wait, spawn
agent 2, wait, ..." — that's sequential execution dressed up as
orchestration.

When in doubt about a particular pair, ask the operator with
`AskUserQuestion` showing the conflict + your proposed wave assignment.

### Concrete patterns

1. **Analyst fan-out.** Kick off separate analyst sub-agents for:
   - market/data research
   - model/LLM research
   - risk & compliance research
   Merge findings into one `spec/<slug>/feature.md`.

2. **Architect fan-out.** For large features, split design into parallel
   investigations (e.g. data layer ADR + ML-serving ADR + risk-engine ADR),
   each in its own sub-agent, then reconcile.

3. **Developer fan-out.** When the task list has independent tasks in
   different crates, spawn one developer sub-agent per crate simultaneously.
   Never spawn two developers into the same file.

4. **Developer + UI-designer parallel.** When a feature has both a backend
   surface and a user-facing surface, spawn the developer (backend crates)
   and the ui-designer (`ui` crate) in the **same Agent tool-use block**.
   They synchronize only on the typed messages defined in `core`. The UI
   side renders against `ui::fixtures` until the developer's real data
   source lands.

5. **Tester fan-out.** Run `rust-validate`, `cargo test`, `rust-bench`,
   `backtest`, and `spec-lint` as parallel sub-agents; the main tester merges
   their outputs into a single report. `spec-lint` is mandatory at the
   tester pre-VERDICT step — exit 0 is required for `VERDICT → PASS`.
   Non-zero routes `HANDOFF → analyst` (or `developer` for source-path
   violations) with the lint output attached.

6. **Presenter is sequential, not fanned out.** Spawn the presenter
   AFTER the tester emits `VERDICT → PASS`. There is no presenter
   fan-out — one feature, one presentation. The presenter may itself
   call multiple skills internally (`present-results`, `verify-anchors`,
   `capture-screenshot`) but the orchestrator spawns it as a single
   agent. Optionally spawn a `preview` mode presenter mid-feature when
   the analyst or architect wants the operator to ratify a direction
   before more work is committed.

Call the Agent tool **once with multiple tool-use blocks in the same message**
to achieve actual concurrency. Sequential `Agent` calls in separate messages
defeat the purpose — they serialize what should be parallel.

**Anti-pattern check** — if you find yourself writing "I'll spawn the X
agent first, then once it lands I'll spawn Y" but X and Y don't share
files / API / Cargo.toml / generated artifacts / operator-decide gates,
you're sequencing for no reason. Spawn both in one block. The runtime
notifies you when each finishes; you don't have to wait.

## Decision framing — durable over quick (operator preference)

**The operator prefers one correct ship over two quick-and-incomplete ships.**
Reworking shipped features is expensive (cognitive context-swap, anchor
re-migration, fresh M-T1+M-DEV+M-FINAL cycles); shipping the right thing
once is cheaper than shipping minimum-viable and patching it. Every agent
in the workflow MUST reflect this preference.

**Concrete applications:**

- **Analyst — Q&D defaults**: when authoring operator-decide questions
  with multiple options, the `(Recommended)` tag goes on the **most
  durable** choice, NOT the cheapest / smallest-blast-radius. If the
  durable choice is meaningfully more expensive, surface the tradeoff
  explicitly: "(Recommended) — costs +1 week vs option B but avoids
  v0.2.0 cleanup brief". Cost-frame in terms of "rework risk over 6
  months" not just wall-clock days.

- **Architect — M-T1 design**: prefer designs that don't spawn v0.2.0
  cleanup briefs. If the architect catches themselves writing
  "MIGRATION: remove at v0.2.0" or "deferred to follow-on" in the
  source, that's a yellow flag — consider whether the scope of v0.1.0
  is wrong. ADR amendments are cheaper than fresh ADRs; designs that
  carry forward unchanged across feature versions are the goal.

- **Developer — implementation choice**: when picking between
  shortcut implementations (e.g. trait `&dyn` vs `impl Trait`, inline
  helper vs lifted module, mock-only vs production-injectable), the
  default is the **lifecycle-cheaper** option, not the **typing-faster**
  option. If you find yourself adding a "MIGRATION:" comment, stop —
  ask the architect whether the design is wrong.

- **Tester — verdict framing**: when surfacing SOFT-PASS or carve-out
  scope, name the **rework cost** explicitly. "Defer to v0.2.0" is
  bookkeeping; "ship a follow-on brief, re-emit N anchors, re-run M
  e2e tests, ~3 days dev + 1 day tester" is the real cost the operator
  uses to decide whether to accept the carve-out.

- **Presenter — deck framing**: when a feature ships with carve-outs,
  the deck explicitly owns the rework debt. List the v0.X+1 follow-on
  brief alongside the v0.X ship, with cost estimate. The operator
  approval is for the v0.X ship AND the debt commitment.

- **Orchestrator — option curation**: when offering the operator
  multiple paths via AskUserQuestion, the `(Recommended)` label goes
  on the durable choice. "Quick win" options exist for situations
  where the architect can PROVE the path won't require future rework
  (ADR Changelog amendment fully covers the change; no carve-out
  surfaced). Default phrasing for cheap choices: "Cheap fallback if
  budget tightens — adds v0.2.0 cleanup commitment".

**Surfaced by the operator 2026-05-28** after a session where multiple
analyst-recommend defaults landed on the cheapest option and the
operator routinely overrode them with the more ambitious choice. The
pattern is consistent enough to codify: when in doubt, bias toward
durable.

## Continuous work — don't pause unnecessarily

**The orchestrator should not artificially throttle after each ship.**
After a commit lands, the default response is to spawn the next-step
agent, not to ask "what's next?" or offer "call it a day" as an
option. The operator stops the session explicitly when they want to;
they don't need orchestrator-side checkpoints to do that.

**Concrete applications:**

- **After any commit**: if there's an unblocked next agent in the
  workflow chain (architect after analyst PASS; developer after
  architect handoff; tester after developer handoff; presenter after
  tester PASS), spawn it. No "ready to proceed?" preamble.

- **Parallel work**: when 1 agent is in flight and another agent
  has independent file-scope, spawn the second one. Don't wait for
  the first to land before kicking off the second. The file-scope
  conflict matrix in § Parallelism rules is the gate, not the
  orchestrator's preference for serial work.

- **End-of-session framing**: do NOT include "call it a day" as a
  default option in AskUserQuestion. The operator will explicitly
  say "stop" / "done" / "tomorrow" when they want to stop. Until
  then, default to "what's the next agent to spawn".

- **Status summaries**: replace "want me to spawn X or hold?" with
  "spawning X now; will report when it lands." If the operator wants
  to override, they say so.

- **"Holding" mode**: legitimate only when there's a real reason
  (file-scope conflict with in-flight agent, dependency on operator
  visual-verify, missing input). After commits, spawn the next agent
  immediately.

- **ALWAYS suggest parallel work** (strengthened 2026-05-29): even
  when an in-flight agent legitimately blocks the linear workflow
  continuation, the orchestrator MUST surface 3-4 parallel-safe
  options in the response. The operator does not like one-track
  waiting. Default response shape when 1 agent is in flight:
  "Holding for X. Meanwhile, parallel-safe options: (1) ... (2) ...
   (3) ...". The file-scope conflict matrix is the gate; if scope is
  clean, the option goes on the list. Apply this even when the
  in-flight agent is the heaviest item — there are ALWAYS analyst
  briefs, audit sweeps, hygiene cleanups, or read-only spec-auditor
  passes that can run in parallel.

**Surfaced by the operator 2026-05-28** after a session where the
orchestrator repeatedly asked "want to call it?" after substantial
ships and offered multi-day waits as an option. The operator's
preference is continuous progress; they don't need orchestrator-side
permission gates.

## Communication contract

- Every sub-agent ends its response with either a `HANDOFF → <agent>` line or
  a `VERDICT → PASS` line. No free-form endings.
- All durable output goes into `spec/` via the `spec-update` skill — nothing
  important lives only in chat.
- Bidirectional loops are allowed: developer may push back to architect;
  architect may push back to analyst; tester may route to any of the above.
  The orchestrator honors those routes rather than forcing linear progress.
- **Human-verification recipe contract** — whenever the orchestrator (or a
  sub-agent through the orchestrator) needs the operator to do something
  out-of-band — run a CLI, eyeball a UI, inspect a report file, populate
  a data cache — the request MUST be a fully self-contained recipe with
  the six sections below. The operator must never have to ask "how do I
  run this?" or "what should I see if it worked?". See
  [memory/feedback_human_verification_recipe.md](.claude/projects/-Users-Vitaliy-Schreibmann-Projects-Privat-trading-trading/memory/feedback_human_verification_recipe.md)
  for the full contract.

  1. **Command(s) to execute** — exact, copy-pasteable bash, fenced. No
     placeholders without example values.
  2. **Step-by-step actions** — numbered list; UI clicks, keystrokes,
     sub-shell probes.
  3. **Expected timing** — wall-clock estimate (so the operator can
     distinguish "still working" from "hung").
  4. **Expected result on success** — verbatim text or specific visual
     state to look for.
  5. **What to do if it fails** — likely failure modes + diagnosis +
     "report back with: <specific artifact>".
  6. **Cleanup** — only if there's state to tear down; omit the section
     otherwise.

  Quality bar: if the operator follows the recipe verbatim and fails,
  that's a recipe defect — fix the recipe, don't blame the operator.
  Confirm flag names via `--help` or source-inspection before emitting.

### Structured handoff envelope

Alongside the prose `HANDOFF →` / `VERDICT →` line, every sub-agent emits a
TOML envelope in a fenced ` ```toml ` block. The envelope makes the
receiving agent's first-pass parse mechanical — it does not need to read the
prose to know what was decided. The prose is still required; the envelope
duplicates the *machine-readable* bits.

Schema:

```toml
[handoff]
from        = "<agent name>"          # analyst | architect | developer | ui-designer | tester | presenter
to          = "<agent name>" | "human"
feature     = "<slug>"                # spec/<slug>/feature.md folder name
trace_refs  = ["REQ-...", ...]        # rows in spec/trace.toml; empty list ok until Phase 1B lands
verdict     = "READY" | "PASS" | "FAIL" | "REGRESSION" | "BLOCKED"
priority    = "P0" | "P1" | "P2"

[inputs]
brief       = "<path to spec-brief artifact, or 'inline'>"
artifacts   = ["<spec path>", ...]    # files the sender read

[outputs]
spec_files  = ["<spec path>", ...]    # files the sender wrote via spec-update
adrs_added  = ["<path to adr/*.md>", ...]   # post-Phase-1A; empty list ok before then

[open_questions]
items = [
  "<one-line question for the next agent>",
  ...
]

[assumptions]
items = [
  "<one-line assumption the sender made — receiver should challenge if false>",
  ...
]
```

Example (architect → developer):

```toml
[handoff]
from        = "architect"
to          = "developer"
feature     = "chart-canvas-overhaul"
trace_refs  = ["REQ-CHART-CANVAS-001", "REQ-CHART-CANVAS-002"]
verdict     = "READY"
priority    = "P1"

[inputs]
brief       = "/tmp/brief-chart-canvas-overhaul.md"
artifacts   = ["spec/chart-canvas-overhaul/feature.md", "spec/architecture.md"]

[outputs]
spec_files  = ["spec/chart-canvas-overhaul/tasks.md"]
adrs_added  = []

[open_questions]
items = [
  "Does the marker-emphasis change require an anchor refresh on btc-2023-1m-sma-cross?",
]

[assumptions]
items = [
  "iced 0.13 stable for the duration of this feature",
  "no new wgpu dependency",
]
```

Adoption rule: every new report after the AGENT.md update date carries an
envelope. Older reports are not retroactively edited. The presenter pre-tick
gate may, in a future change, refuse to ship a feature whose latest sub-
agent reports lack envelopes — track adoption first, enforce later.

## Queue pre-flight reconciliation sweep (2026-05-29 contract)

**Before promoting any Queue entry to Active, the orchestrator MUST
verify the feature folder's frontmatter `status` against the Queue
row text.** Per the weekly-retro-2026-05-27-to-2026-05-29 finding:
the audit caught stale Queue→Active text on 3 consecutive audits
(v2.5 TCN near-miss save 2026-05-28; v25a-patchtst earlier;
v3-llm-forecaster Queue row says "moved to Active" while shipped).

**Mandatory steps before Queue → Active promotion:**

1. Grep for the slug in `spec/<slug>/feature.md` frontmatter; read
   the `status:` field.
2. If `status: shipped` or `status: shipped (retired)` — the Queue
   entry is STALE; DO NOT promote. Instead:
   - Update the Queue text to reflect the shipped state (e.g.
     "shipped 2026-MM-DD; see Recent")
   - Pick a different track or surface a different option to operator
3. If `status: draft` / `status: proposed` / `status: in-progress` —
   safe to promote; proceed with analyst M-A5 light-touch refresh.
4. The analyst-halt protocol (2026-05-28 lesson) is the LAST line of
   defense — orchestrator should catch this BEFORE spawning analyst.

This is the same pattern the analyst correctly enforced 2026-05-28
when refusing to overwrite v2.5 TCN artifacts. Codifying it at
orchestrator level prevents wasted analyst cycles.

**Automated pre-flight (2026-05-30).** Run
`python3 scripts/queue_staleness_check.py` at session start. Exit 0 =
clean (silent); exit 1 = drift — the script emits a markdown table on
stdout naming each stale slug + suggested fix; paste it verbatim into
the session header and reconcile before promoting. Exit >= 2 = script
failure (stderr); investigate the backlog parse. The manual 4-step
check below remains the fallback when the script can't reason about an
entry (suppress a single stub inline with `# noqa: queue-staleness`).

### Pending operator-verification ledger (2026-05-29 contract)

The single source of truth for operator-run recipes that survive
session boundaries is
[`spec/dev-notes/operator-side-pending-ledger.md`](spec/dev-notes/operator-side-pending-ledger.md)
(orchestrator-maintained, append-only). Its schema is enforced by
`scripts/operator_ledger_check.py` (Python stdlib; exit 0 clean / 1 on
schema violation or stale-FAILED escalation / >= 2 on script failure).
Run it at session pre-flight alongside the Queue-staleness sweep:

    python3 scripts/operator_ledger_check.py        # uses today's date
    python3 scripts/operator_ledger_check.py --today 2026-05-29   # deterministic

FAILED rows older than 7 days escalate (exit 1); FAILED rows within the
window emit a soft carry-over line for the session header. Every FAILED
row MUST cite a follow-up `spec/dev-notes/*.md` investigation in its
Notes cell (Q-LED-NOTE). See
[`spec/operator-ledger-schema-lint/feature.md`](spec/operator-ledger-schema-lint/feature.md).

## The vibe-coding loop

**Before every sub-agent delegation**, the orchestrator assembles a brief:

```bash
scripts/spec_brief.py <slug> --out /tmp/brief-<slug>.md
```

and includes the brief path in the delegation prompt (e.g. "Read your brief
at `/tmp/brief-<slug>.md` first."). This is the mechanism that keeps sub-
agents off `spec/architecture.md` (296 KB) and on the curated 3-5k-token
slice they actually need. For greenfield analyst work where no feature
folder exists yet, the brief is skipped — analyst reads `CLAUDE.md` +
`product.md` + the `INV-*` rows from `spec/trace.toml` instead. See
`.claude/skills/spec-brief/SKILL.md` for the full contract.

The loop:

1. User states intent in plain language.
2. Orchestrator opens/updates `spec/product.md` if needed, then:
   - spawns **analyst** (possibly fanned-out) to turn intent into a feature brief;
   - (greenfield case: no spec-brief; analyst reads invariants + product).
3. Orchestrator reads the analyst's handoff envelope, then:
   - generates `spec_brief.py <slug>` and includes the path in the prompt;
   - spawns **architect** (possibly fanned-out) for design + task list.
4. Orchestrator regenerates the brief (analyst+architect outputs now landed)
   and spawns **developer** sub-agents in parallel across independent
   backend tasks. **In the same tool-use block**, if the feature has a UI
   surface, also spawns **ui-designer** for the `ui` crate work.
5. Orchestrator regenerates the brief one more time, then spawns **tester**
   which fans out into parallel
   validate/test/bench/backtest and merges into one report;
6. Orchestrator reads the verdict:
   - PASS → spawn **presenter** for `release` mode. Presenter assembles
     `spec/<slug>/presentations/<slug>-<date>.md`, runs real bins, embeds
     verification matrix + numbers, lists open decisions. Hand the file
     path back to the user with the approval block. Wait for the
     operator's tick.
   - FAIL / REGRESSION → route to the agent named in the report and loop
     (UX/visual regressions route to **ui-designer**).
7. **Operator approval gate:** when the user approves the presentation,
   the feature ships (status `→ shipped`). If the operator approves with
   notes, append the notes to the presentation's feedback log and route
   to the relevant agent for follow-up. If the operator rejects, the
   presenter routes back to the agent that owns the failure mode.
8. At every step, the agent MUST write to the right spec file. The chat is a
   view, not a store.

### Orchestrator-owned brief regeneration

The brief is **not** generated once per feature; it's regenerated before
each sub-agent delegation because each preceding agent's output may have
materially changed the relevant spec. The contract:

- Generate the brief immediately before calling the sub-agent.
- Write to a path including the date or step number so reruns are
  diffable: `/tmp/brief-<slug>-<step>.md` (e.g. `-arch`, `-dev`, `-test`).
- Pass the path in the delegation prompt's first line.
- If `spec_brief.py` reports >10k tokens, do not silently truncate —
  surface the warning, file a `spec-auditor` triage item, and consider
  whether the feature has outgrown a single-shot delegation.

The orchestrator never reads `spec/architecture.md` directly either —
it lets `spec_brief.py` do the relevant-section extraction.

## When does ui-designer get involved?

- Any feature that adds a new screen, panel, widget, modal, or view.
- Any feature that adds new user-visible text, even if no new screen.
- Any change that affects the ops cockpit's live view (P&L, positions, log).
- Any change to alerts, confirms, or destructive-action flows.
- Periodic **consistency audits** — even with no feature in flight, the
  orchestrator may spawn ui-designer to scan for theme/string drift and
  produce a `spec/<slug>/reports/ui-debt-<date>.md`.

If a feature is purely backend (data ingestion plumbing, model training
script, no operator-visible change), skip ui-designer.

## When does presenter get involved?

- **Always after `VERDICT → PASS`** for any feature the operator will
  ship or use directly. Even backend features that have no UI surface
  still get a presentation — the operator needs to know what changed
  and approve.
- **Optionally mid-feature** in `preview` mode, when the analyst or
  architect wants the operator to ratify a non-trivial design choice
  before development commits to it. Examples: "we're considering
  Postgres vs SQLite — here's the tradeoff, please pick"; "v2 RL
  strategy will need a GPU budget — approve the cost?".
- **Skip the presenter** for tiny one-line fixes, doc-only changes,
  refactors with no behavior change, and dependency bumps. Presenter
  ceremony costs more than it saves on those.

The presenter is the only agent that addresses the human in
presentation form. Other agents may write reports the operator reads,
but those are technical artifacts — the presenter is the agile
"sprint review" face of the whole team.

## Skills catalog

| Skill                | Purpose                                                       |
|----------------------|---------------------------------------------------------------|
| `rust-build`         | `cargo check` / `build` pipeline                              |
| `rust-test`          | Full test matrix + report generation                          |
| `rust-validate`      | fmt, clippy, audit, deny, docs                                |
| `rust-bench`         | Criterion benchmarks with baseline diffs                      |
| `backtest`           | Historical strategy simulation                                |
| `verify-anchors`     | Regression-gate the body-SHA anchors in `spec/anchors.toml`   |
| `present-results`    | Assemble a `spec/<slug>/presentations/<slug>-<date>.md` from spec + tests + live bin runs |
| `capture-screenshot` | Capture (or operator-instruct) a UI screenshot                |
| `spec-update`        | Safe writer for `spec/` files                                 |
| `cockpit-smoke`      | Orchestrator-only pre-tick gate: boots fixtures cockpit for 7s + greps stderr for panics (per ui-quality-gate-overhaul M1-A) |

Agents invoke skills; the orchestrator does not need to call skills directly
unless operating without sub-agents.

## Tooling — `scripts/`

Small utilities that several agents share. Use them instead of inline
Python or hand-typed pipelines.

| Script                                  | Purpose                                                       | Caller                       |
|-----------------------------------------|---------------------------------------------------------------|------------------------------|
| `scripts/hash_report.py`                | Body-only SHA-256 of a YAML-front-mattered report file        | tester, developer, architect |
| `scripts/verify_anchors.sh`             | Verify all 9 anchors in `spec/anchors.toml`                   | tester (mandatory gate)      |
| `scripts/precheck.sh`                   | Stdlib-name clash check + task-tick summary                   | architect, orchestrator      |
| `scripts/spec_brief.py`                 | Generate per-feature curated context brief for sub-agent prompts | orchestrator                 |
| `scripts/spec_lint.py`                  | Mechanical lint of `spec/` (frontmatter, trace, hygiene)      | spec-auditor (via `spec-lint` skill) |
| `scripts/check_presentation.sh`         | Pre-tick guard for `spec/<slug>/presentations/*.md` shape     | presenter (via `present-results` skill) |
| `scripts/capture_screenshot.sh`         | Darwin `screencapture` wrapper for cockpit/binary output      | tester, presenter (via `capture-screenshot` skill) |
| `scripts/pre_stage_anchors.sh`          | Stage candidate anchor SHAs from a backtest run               | tester, architect (anchor refresh) |
| `scripts/prune_backtest_duplicates.sh`  | Collapse duplicate backtest reports in `spec/*/reports/`      | tester (via `rust-test`, `verify-anchors`, `backtest` skills) |
| `scripts/check_no_secrets_in_llm_artifacts.sh` | Guard: LLM artifacts contain no secrets                | tester, regression gate      |
| `scripts/check_no_clocks_in_ui_tests.sh`| Guard: UI tests have no wall-clock dependency                 | tester, regression gate      |

For the orchestrator-only `scripts/orch_*` set (cursor automation,
screencapture, cockpit on/off, supplement-log, determinism check, TCC
probe, PNG crop), see
[`spec/dev-notes/orchestrator-tooling-2026-05-12.md`](spec/dev-notes/orchestrator-tooling-2026-05-12.md).
Sub-agents must NOT call those — they wrap capabilities scoped to the
orchestrator's lane.

`spec/anchors.toml` is the single source of truth for locked anchor SHAs
— never duplicate hashes into feature/task/report files. Update only via
architect approval; tester locks new entries.

## Process discipline (lessons from v0 → v1.5a)

These rules exist because we paid for them. Each one maps to a real
incident and a concrete tooling gate:

1. **Honest tick.** The developer agent MUST NOT mark a task `[x]`
   without citing three things: (a) the file:line where the change
   landed, (b) the test command exercising it, (c) the test-output
   line proving it passed. If you cannot cite all three, leave the
   tick blank and finish with `HANDOFF → tester (verify and tick)`.
   *Why:* every version v0/v0.5/v1/v1.5a had a developer round that
   ticked tasks before the work was done.

2. **Tester owns `T_FINAL_*` ticks.** The developer never ticks the
   `T_FINAL_*` rows. Only the tester does, and only after `VERDICT →
   PASS` AND `verify-anchors` PASS. If the dev list ends with an
   unticked `T_FINAL_*`, the developer has finished correctly — that
   is the handoff.

3. **Anchor gate.** Any tester run that touched `crates/strategy/`,
   `crates/audit/`, `crates/exec/`, `crates/backtest/`, or report
   rendering MUST run `verify-anchors`. A single FAIL routes
   `HANDOFF → developer` with the body diff. The 9 anchors live in
   `spec/anchors.toml`; nowhere else.

4. **Body-vs-front-matter discipline.** Anything that may differ
   between two equivalent runs (timestamps, wall-clock, host, pid,
   git commit, generated:, data_source variants) belongs in YAML
   front-matter — never in the body. The body is what gets hashed.
   *Why:* HF-1 (`wall_clock_s`) and T715 (`data_source` string)
   each cost a round.

5. **Determinism non-negotiables** (developer-agent checklist):
   - No `SystemTime::now()` / `Instant::now()` reachable from a
     backtest replay path. Inject a clock.
   - No `f64` in money math. `rust_decimal::Decimal` + `Money<C>`
     newtype only.
   - Microsecond fractional-second timestamps in the audit DB —
     `Rfc3339` second-precision causes SQLite ORDER BY ties.
   - All RNGs `ChaCha20Rng::from_seed(...)`. No `thread_rng`.
   - HashMap iteration sorted before any cross-run comparison.

6. **UI brief pre-tick gate — `cockpit-smoke`.** Every UI brief's
   evaluator `VERDICT → PASS` triggers the
   [`cockpit-smoke`](.claude/skills/cockpit-smoke/SKILL.md) skill
   before the presenter pre-tick gate runs. Skill exit `0` →
   orchestrator continues to the presenter. Skill exit `1` → block
   presenter, route `HANDOFF → developer` with the skill's
   panic-grep output attached to the dev brief. Cadence is
   **always-on** (operator ratified) — not scoped to
   `crates/ui/src/widgets/` / `crates/ui/src/screens/` touches.
   Any spec slug whose tester report cites `cargo test -p ui` runs
   the skill. Invocation boundary: orchestrator-only per
   [`## Capability boundaries`](#capability-boundaries) table row
   `cargo run --bin cockpit with a live window`. *Why:* the F1
   first-frame `fill_quad`/`unreachable!()` panic shipped past the
   267-test panel-snapshot suite because no harness exercised the
   iced render path against a live tiny-skia surface
   (see `spec/cockpit-render-regression/feature.md` for the
   incident). This rule closes that gap.

7. **Spec-shape pre-tick gate — `spec-lint`.** Every feature's tester run
   AND every presenter pre-tick MUST end with a clean
   [`spec-lint`](.claude/skills/spec-lint/SKILL.md) (categories
   `dead-link`, `missing-frontmatter`, `orphan-feature`, `bad-anchor`,
   `unreferenced-anchor`, `shipped-no-tests`, `trace-broken-path`,
   `adr-not-registered`). Invocation: `uv run scripts/spec_lint.py`
   (PEP-723 header pins Python ≥ 3.11; macOS system Python 3.9 will
   fail). Exit `0` → continue. Non-zero → block the verdict / pre-tick
   and route `HANDOFF → analyst` (link / frontmatter / orphan
   violations) or `HANDOFF → developer` (source-path violations under
   `trace-broken-path`). Pairs with `verify-anchors` (content hashes)
   to give the project two mechanical gates: shape stability here,
   content stability there. *Why:* the 2026-05-16 spec-hygiene
   remediation surfaced 708 dead intra-spec links and 11 shipped-
   features-without-test-reports — both classes are mechanically
   detectable, and without a wired-in gate they re-accumulate.

## Guardrails

- **Never** let an agent silently diverge from `spec/architecture.md`. Drift
  is either a spec update or a handoff — never both missing.
- **Never** accept a tester report with missing sections; reject and re-run.
- **Never** ship on a REGRESSION verdict without a human "proceed anyway".
- **Never** use `unsafe` Rust without a `// SAFETY:` comment.
- **Never** commit secrets; exchange keys live in env vars or a secret store
  defined in `spec/architecture.md`.
- **Never** create a git worktree, feature branch, or `claude/<slug>` branch.
  All work commits directly to `main`. See `## Branch & worktree policy`
  at the top of this file.
- **Never** let a sub-agent run `git commit` or `git push`. Sub-agents
  write files; the orchestrator commits + pushes.

## Capability boundaries (orchestrator vs. sub-agent)

Adopted 2026-05-12 after the chart-canvas-overhaul retrospective
([spec/dev-notes/ui-testing-direction-2026-05-12.md](spec/dev-notes/ui-testing-direction-2026-05-12.md)).
Sub-agents are **context tools, not capability tools**. Their toolset is a
subset of the orchestrator's. When a sub-agent's sandbox blocks a capability
the orchestrator has, the sub-agent must escalate, not rationalize.

### Capability map

| Capability | Owner | Sub-agents allowed? |
|---|---|---|
| `cargo fmt`, `cargo clippy`, `cargo test` (pure Rust) | sub-agent | yes |
| `verify_anchors.sh` | sub-agent | yes |
| `spec_lint.py` (via `uv run` or `python3 ≥ 3.11`) | sub-agent | yes |
| `rust-build`, `rust-validate` skills | sub-agent | yes |
| `spec-update` writes to `spec/<slug>/` | sub-agent | yes |
| `cargo run --bin cockpit` with a live window | **orchestrator** | **no** |
| `screencapture` of the running app | **orchestrator** | **no** |
| `osascript`, `cliclick`, Swift `CGWarp` cursor automation | **orchestrator** | **no** |
| Concluding "the bug is X" from live-app instrumentation | **orchestrator** | **no** |
| Adjudicating disagreements between sibling sub-agents | **orchestrator** | **no** |
| Visual approval / rejection of UI | **operator** | **no** |

### Test-runner / evaluator split

The single `tester` role is split into two:

- **test-runner** (write-allowed): runs `rust-test`, `rust-validate`,
  `verify_anchors`, dumps raw output to
  `spec/<slug>/reports/test-run-<ts>.log`. No verdict, no prose.
- **evaluator** (read-only): fresh context that never saw the developer
  diff. Only `Read` + `Bash(grep|wc|sha256sum|cat)`. Reads the run log + any
  cited artifact screenshots. Writes
  `spec/<slug>/reports/evaluation-<ts>.md`. VERDICT → PASS/FAIL/REGRESSION
  emits from the evaluator, never from the test-runner. Mirrors Anthropic's
  reference harness ([cwc-long-running-agents](https://github.com/anthropics/cwc-long-running-agents))
  to break the "agents skew positive when grading their own work" failure
  mode.

Once PreToolUse hooks are wired (week 3 of the
[ui-testing-direction adoption plan](spec/dev-notes/ui-testing-direction-2026-05-12.md)),
the evaluator's `Write` on the evaluation file is default-FAIL unless its
read trace contains the run log AND every cited artifact. Until those hooks
land, the contract is procedural — the evaluator agent's brief enforces it
by instruction.

### Architect = hypothesis only

Architects author **hypotheses with explicit falsifiers** ("if X, then
measurement Y will show Z"). They do NOT:

- Run instrumentation that requires a display server / GPU / running window.
- Conclude "the bug is X" without a citation to an orchestrator-run
  empirical test that refused to falsify.

Hypotheses without orchestrator-run falsification are first-class spec
artifacts — they appear in `feature.md ## Hypothesis register` and the
orchestrator picks which to falsify first. The chart-canvas-overhaul
"iced has a half-scale canvas bug" misdiagnosis (1.5 dev-days of dead
code) is the prior incident this rule prevents.

### Parallelism caveat

The parallelism rules in `## Parallelism rules` still hold for analyst
fan-out and orchestrator-coordinated dev/ui-designer splits. But:

- **Default to sequential** dev → ui-designer → orchestrator when in
  doubt. Cognition's "[Don't Build Multi-Agents](https://cognition.ai/blog/dont-build-multi-agents)"
  documents the silent-divergence failure mode: parallel sub-agents have
  no view of each other's reasoning. We hit it in
  chart-canvas-overhaul M7 when both agents needed to patch
  `cockpit.rs:158` for screenshot capture and coordinated via tasks.md.
- **Most coding tasks involve fewer truly parallelizable tasks than
  research** ([Anthropic — multi-agent research](https://www.anthropic.com/engineering/multi-agent-research-system)).
  When the orchestrator can't articulate the lane split explicitly in the
  spawn brief, default to sequential.

## When NOT to use sub-agents

- Trivial one-file edits where spinning up an agent costs more than it saves.
- Purely conversational questions about the code.
- Quick compile checks after a one-line change.
- **Any task that requires a display server, GPU, screenshot, or window
  automation.** Per the capability map above, the orchestrator owns these.

Use direct tools for those; reserve agents for work big enough to justify the
handoff overhead.
