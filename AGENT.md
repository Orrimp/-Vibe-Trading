# AGENT.md — Orchestration & Workflow

This document is the contract for how the six specialist agents collaborate
on the Rust crypto trading agent. It is **required reading** for any Claude
session acting as the orchestrator.

> This project uses sub-agents and expects them to run in **parallel** whenever
> their work is independent. Sequential handoffs are only for dependent work.

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

The orchestrator (the main Claude session) MUST spawn sub-agents in parallel
whenever their tasks are independent. Concrete patterns:

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

5. **Tester fan-out.** Run `rust-validate`, `cargo test`, `rust-bench`, and
   `backtest` as parallel sub-agents; the main tester merges their outputs
   into a single report.

6. **Presenter is sequential, not fanned out.** Spawn the presenter
   AFTER the tester emits `VERDICT → PASS`. There is no presenter
   fan-out — one feature, one presentation. The presenter may itself
   call multiple skills internally (`present-results`, `verify-anchors`,
   `capture-screenshot`) but the orchestrator spawns it as a single
   agent. Optionally spawn a `preview` mode presenter mid-feature when
   the analyst or architect wants the operator to ratify a direction
   before more work is committed.

Call the Agent tool **once with multiple tool-use blocks in the same message**
to achieve actual concurrency. Sequential calls defeat the purpose.

## Communication contract

- Every sub-agent ends its response with either a `HANDOFF → <agent>` line or
  a `VERDICT → PASS` line. No free-form endings.
- All durable output goes into `spec/` via the `spec-update` skill — nothing
  important lives only in chat.
- Bidirectional loops are allowed: developer may push back to architect;
  architect may push back to analyst; tester may route to any of the above.
  The orchestrator honors those routes rather than forcing linear progress.

## The vibe-coding loop

1. User states intent in plain language.
2. Orchestrator opens/updates `spec/product.md` if needed, then:
   - spawns **analyst** (possibly fanned-out) to turn intent into a feature brief;
3. Orchestrator reads the analyst's handoff, then:
   - spawns **architect** (possibly fanned-out) for design + task list;
4. Orchestrator spawns **developer** sub-agents in parallel across independent
   backend tasks. **In the same tool-use block**, if the feature has a UI
   surface, also spawns **ui-designer** for the `ui` crate work.
5. Orchestrator spawns **tester** which fans out into parallel
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

Agents invoke skills; the orchestrator does not need to call skills directly
unless operating without sub-agents.

## Tooling — `scripts/`

Small utilities that several agents share. Use them instead of inline
Python or hand-typed pipelines.

| Script                         | Purpose                                                  | Caller                       |
|--------------------------------|----------------------------------------------------------|------------------------------|
| `scripts/hash_report.py`       | Body-only SHA-256 of a YAML-front-mattered report file   | tester, developer, architect |
| `scripts/verify_anchors.sh`    | Verify all 9 anchors in `spec/anchors.toml`              | tester (mandatory gate)      |
| `scripts/precheck.sh`          | Stdlib-name clash check + task-tick summary              | architect, orchestrator      |

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

## Guardrails

- **Never** let an agent silently diverge from `spec/architecture.md`. Drift
  is either a spec update or a handoff — never both missing.
- **Never** accept a tester report with missing sections; reject and re-run.
- **Never** ship on a REGRESSION verdict without a human "proceed anyway".
- **Never** use `unsafe` Rust without a `// SAFETY:` comment.
- **Never** commit secrets; exchange keys live in env vars or a secret store
  defined in `spec/architecture.md`.

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
