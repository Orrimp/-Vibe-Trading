# AGENT.md — Orchestration & Workflow

This document is the contract for how the five specialist agents collaborate
on the Rust crypto trading agent. It is **required reading** for any Claude
session acting as the orchestrator.

> This project uses sub-agents and expects them to run in **parallel** whenever
> their work is independent. Sequential handoffs are only for dependent work.

## The five agents

| Agent        | Model  | File                                  | Primary role                              |
|--------------|--------|---------------------------------------|-------------------------------------------|
| analyst      | opus   | `.claude/agents/analyst.md`           | Research, requirements, critique          |
| architect    | opus   | `.claude/agents/architect.md`         | System design, crate layout, tradeoffs    |
| developer    | sonnet | `.claude/agents/developer.md`         | Rust implementation, tests alongside code |
| ui-designer  | opus   | `.claude/agents/ui-designer.md`       | iced UI: implement + consistency + human-friendliness |
| tester       | sonnet | `.claude/agents/tester.md`            | Build, test, validate, backtest, report   |

Opus for deep thinking (analyst, architect, ui-designer — UI is hard).
Sonnet for high-throughput execution (developer, tester).

## Canonical workflow

```
                  ┌─────────────┐
                  │   analyst   │  opus  — requirements, research
                  └──────┬──────┘
                         │ HANDOFF (spec/features/<slug>.md)
                         ▼
                  ┌─────────────┐
                  │  architect  │  opus  — design, task breakdown
                  └──────┬──────┘
                         │ HANDOFF (spec/tasks/<slug>.md)
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
                │  PASS → ship           │
                │  FAIL → developer      │
                │       or ui-designer   │
                │  REGRESSION:           │
                │   structural → architect │
                │   strategy   → analyst   │
                │   UX/visual  → ui-designer │
                └────────────────────────┘
```

Feedback edges are **first-class**. The tester's report is not a terminator —
it is an input to whichever agent owns the failure mode. UI-designer and
developer run **in parallel** whenever a feature has both backend and UI
work; they synchronize only on shared types in the `core` crate.

## Parallelism rules

The orchestrator (the main Claude session) MUST spawn sub-agents in parallel
whenever their tasks are independent. Concrete patterns:

1. **Analyst fan-out.** Kick off separate analyst sub-agents for:
   - market/data research
   - model/LLM research
   - risk & compliance research
   Merge findings into one `spec/features/<slug>.md`.

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
   - PASS → summarize for the user, ask what's next.
   - FAIL / REGRESSION → route to the agent named in the report and loop
     (UX/visual regressions route to **ui-designer**).
7. At every step, the agent MUST write to the right spec file. The chat is a
   view, not a store.

## When does ui-designer get involved?

- Any feature that adds a new screen, panel, widget, modal, or view.
- Any feature that adds new user-visible text, even if no new screen.
- Any change that affects the ops cockpit's live view (P&L, positions, log).
- Any change to alerts, confirms, or destructive-action flows.
- Periodic **consistency audits** — even with no feature in flight, the
  orchestrator may spawn ui-designer to scan for theme/string drift and
  produce a `spec/reports/ui-debt-<date>.md`.

If a feature is purely backend (data ingestion plumbing, model training
script, no operator-visible change), skip ui-designer.

## Skills catalog

| Skill           | Purpose                                   |
|-----------------|-------------------------------------------|
| `rust-build`    | `cargo check` / `build` pipeline          |
| `rust-test`     | Full test matrix + report generation      |
| `rust-validate` | fmt, clippy, audit, deny, docs            |
| `rust-bench`    | Criterion benchmarks with baseline diffs  |
| `backtest`      | Historical strategy simulation            |
| `spec-update`   | Safe writer for `spec/` files             |

Agents invoke skills; the orchestrator does not need to call skills directly
unless operating without sub-agents.

## Guardrails

- **Never** let an agent silently diverge from `spec/architecture.md`. Drift
  is either a spec update or a handoff — never both missing.
- **Never** accept a tester report with missing sections; reject and re-run.
- **Never** ship on a REGRESSION verdict without a human "proceed anyway".
- **Never** use `unsafe` Rust without a `// SAFETY:` comment.
- **Never** commit secrets; exchange keys live in env vars or a secret store
  defined in `spec/architecture.md`.

## When NOT to use sub-agents

- Trivial one-file edits where spinning up an agent costs more than it saves.
- Purely conversational questions about the code.
- Quick compile checks after a one-line change.

Use direct tools for those; reserve agents for work big enough to justify the
handoff overhead.
