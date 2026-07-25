# AGENT.md — BMAD Orchestration & Workflow

This document is the contract for how work gets done since the **BMAD-METHOD
v6.10.0 migration** (completed 2026-07-25; plan:
[`docs/dev-notes/bmad-migration-plan-2026-07-24.md`](docs/dev-notes/bmad-migration-plan-2026-07-24.md)).
It is **required reading** for any Claude session acting as the orchestrator.

> **File precedence for AI agents**: read [README.md](README.md) first
> (project orientation + status snapshot + quickstart), then
> [CLAUDE.md](CLAUDE.md) (coding rules + non-negotiables), then this file
> (orchestration). AGENT.md is the third file in the chain; the BMAD
> planning/implementation artifacts under `_bmad-output/` are fourth.

> The six legacy `.claude/agents/*` definitions are **RETIRED** (archived at
> [`docs/archive/pre-bmad-agents/`](docs/archive/pre-bmad-agents/)). Their
> project knowledge lives on in the committed overrides under
> [`_bmad/custom/`](_bmad/custom/) and in the non-negotiables
> ([CLAUDE.md](CLAUDE.md)). This file maps the old roles onto the BMAD seams.

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

## Persona mapping (legacy → BMAD)

| Legacy agent (retired) | BMAD seam | Fidelity | Project knowledge lives in |
|---|---|---|---|
| **analyst** | `bmad-agent-analyst` (+ `bmad-prd`, `bmad-domain-research`, `bmad-market-research`) | clean | `_bmad/custom/bmad-agent-analyst.toml` |
| **architect** | `bmad-agent-architect` (+ `bmad-architecture`, `bmad-create-epics-and-stories`, `bmad-check-implementation-readiness`) | clean | `_bmad/custom/bmad-agent-architect.toml` |
| **developer** | `bmad-agent-dev` (+ `bmad-create-story`, `bmad-dev-story`) — **also carries the tester's gate-EXECUTION half** (menu items RB/RT/RV/VA/BT drive `rust-build`/`rust-test`/`rust-validate`/`verify-anchors`/`backtest`) | clean | `_bmad/custom/bmad-agent-dev.toml` |
| **ui-designer** | `bmad-agent-ux-designer` (+ `bmad-ux`) — BMAD's ux persona is design-only, so the override adds the rust build/test menu (our ui-designer owns `crates/ui` code end to end) | clean | `_bmad/custom/bmad-agent-ux-designer.toml` |
| **tester** | **SPLIT — no BMAD QA persona exists.** Static review-discipline half → `bmad-code-review` (fresh context, adversarial layers, the report-template + anchor/spec-lint gates + verify-before-route rule). Dynamic gate-execution half → the dev persona's harness menu (row above). e2e authoring → `bmad-qa-generate-e2e-tests` (stock). | role-preserved-via-workflow | `_bmad/custom/bmad-code-review.toml` |
| **presenter** | `bmad-agent-tech-writer` (+ the `present-results` skill + `scripts/check_presentation.sh`) — **DELTA vs the plan's § 6 table**, which mapped presenter → `bmad-agent-pm`; the Phase-5a dispatch landed it on tech-writer (deck-assembly + plain-language translation is Paige's structural match; `bmad-agent-pm` stays stock/unmapped). `bmad-sprint-status` + `bmad-retrospective` remain standalone by name. | role-preserved | `_bmad/custom/bmad-agent-tech-writer.toml` |

**No-BMAD-twin specialists (kept, invoked by name, charters committed):**

- **spec-auditor** — read-only drift audit (anchors/trace/orphans/dead links);
  runs the re-founded `spec_lint` + `verify_anchors` and emits a dated
  `docs/dev-notes/audit-*.md`. Charter:
  [`_bmad/custom/spec-auditor-charter.md`](_bmad/custom/spec-auditor-charter.md).
  (`bmad-sprint-status` covers risk callouts, NOT anchor/trace drift.)
- **ui-debugger** — cockpit render/behavior bug fixer at the rendered-PIXEL
  layer. Charter:
  [`_bmad/custom/ui-debugger-charter.md`](_bmad/custom/ui-debugger-charter.md);
  method: [`docs/dev-notes/iced-ui-render-verification.md`](docs/dev-notes/iced-ui-render-verification.md).
  Route a UI **bug** ("no graph", "blank panel", a mystery UI-test failure)
  here; route UI **design/implementation** to the ux-designer persona.
- **researcher** — dormant (the `research/` KB is complete, 900/900). Charter:
  [`_bmad/custom/researcher-charter.md`](_bmad/custom/researcher-charter.md).

## The BMAD cycle

**One workflow per fresh chat/agent context** — a workflow's output artifact
(story file, review findings, sprint-status sync) is its handoff; the next
workflow starts clean and reads the artifact, never inherited chat state.
[`bmad-help`](.claude/skills/bmad-help/SKILL.md) is the entry point when the
next step is unclear.

```
planning (as needed):
  bmad-prd ──► bmad-architecture ──► bmad-create-epics-and-stories ──► bmad-sprint-planning
                     │ (ADRs land in planning-artifacts/architecture/decisions/, AD-18 atomic)
delivery (per story):
  bmad-sprint-status          — where does the board stand? what's next?
        │
  bmad-create-story           — story file with ALL context the dev needs
        │                       (_bmad-output/implementation-artifacts/{epic}-{story}-{slug}.md)
  bmad-dev-story              — implement tasks/subtasks, tests alongside code
        │                       (dev persona drives rust-build/test/validate + verify-anchors)
  bmad-code-review            — fresh-context adversarial review + THE GATES:
        │                       verify_anchors 119/119, spec_lint PASS, clippy -D warnings
        │                       (re-run yourself; sub-agent claims and clippy's cache both lie)
  story → done                — triad move: story Status + trace.toml state + CHANGELOG line
        │                       (spec_lint enforces all three legs)
  bmad-retrospective          — per epic, on demand
```

Verdict routing keeps its old semantics: review **PASS** → the story flips
`review` → `done` (triad move) and the orchestrator commits; **FAIL** routes
back to the dev persona with findings; a **REGRESSION** finding routes to the
architect persona (structural) or analyst persona (strategy/product) and
**blocks ship absent an explicit human override**. Operator-facing decks
(the old presenter surface) come from the tech-writer persona +
`present-results`, with `bash scripts/check_presentation.sh` as the pre-tick
gate — approval boxes ship un-ticked, always; the human operator is the only
one who ticks.

### Orchestrator duties (unchanged by the migration)

1. **Git authority.** The orchestrator commits and pushes; sub-agents and
   personas only write files (see Branch & worktree policy). LFS note: if
   push fails on `lfs.locksverify`, bypass once with
   `git -c lfs.locksverify=false push`.
2. **Gates before every commit** — run them yourself, in the same session:
   `bash scripts/verify_anchors.sh` (**119/119**), `python3
   scripts/spec_lint.py` (**PASS (0 violations)**), `python3
   scripts/adr_registry_check.py --pre-commit` when `decisions/` changed,
   plus `cargo fmt --check` / `clippy -D warnings` / affected tests when code
   changed.
3. **Independent verification.** Sub-agent PASS/FAIL claims are hypotheses:
   re-run the gate before routing on it (force a fresh clippy pass — its
   cache lies both ways; `#[ignore]`'d-test failures are not gated — call
   them out explicitly, never silently pass or block on them).
4. **Watch recipes for long jobs.** Any process plausibly over 2 minutes
   (training, full-year backtests, criterion, workspace builds) is launched
   with a copy-pasteable `watch -n N '<probe>'` block in the SAME message —
   pgrep-derived PID, a forward-progress line, defensive against "not
   running yet".
5. **Human-verification recipe contract** — whenever the operator must do
   something out-of-band (run a CLI, eyeball a UI, inspect a report,
   populate a data cache), the request MUST be a fully self-contained recipe
   with the six sections below. The operator must never have to ask "how do
   I run this?" or "what should I see if it worked?".

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

### Structured handoff envelope (story-keyed)

Alongside its prose conclusion, every sub-agent/workflow run ends with a
`HANDOFF → <seam>` or `VERDICT → PASS|FAIL|REGRESSION` line plus a TOML
envelope so the receiving context's first-pass parse is mechanical. The
schema survives the migration with story-keyed fields:

```toml
[handoff]
from        = "<seam>"       # analyst | architect | dev | ux-designer | code-review | tech-writer | spec-auditor | ui-debugger
to          = "<seam>" | "human"
story       = "<{epic}-{story}-{slug}>"   # file under _bmad-output/implementation-artifacts/
trace_refs  = ["REQ-...", ...]            # [[req]] ids in planning-artifacts/trace.toml; [] ok
verdict     = "READY" | "PASS" | "FAIL" | "REGRESSION" | "BLOCKED"
priority    = "P0" | "P1" | "P2"

[inputs]
brief       = "<path to brief artifact, or 'inline'>"
artifacts   = ["<path>", ...]             # files the sender read

[outputs]
files       = ["<path>", ...]             # files the sender wrote
adrs_added  = ["_bmad-output/planning-artifacts/architecture/decisions/NNNN-*.md", ...]

[open_questions]
items = ["<one-line question for the next seam>", ...]

[assumptions]
items = ["<one-line assumption — receiver should challenge if false>", ...]
```

BMAD has no equivalent hand-off schema; this contract is ours and stays.

## Parallelism rules

**Default to parallel.** The orchestrator MUST spawn sub-agents in parallel
whenever their tasks are independent. Sequential execution is the exception —
justified only by a real file-scope conflict, an explicit dependency edge, or
an unanswered operator-decide gate.

When the operator hands over a multi-item agenda, the orchestrator's FIRST
action is to partition it into waves using the conflict matrix below and
present the wave plan before spawning.

### File-scope conflict matrix (pre-spawn checklist)

Before spawning any wave, for every (agent_i, agent_j) pair answer YES/NO:

1. **Touches the same file?** Two agents editing the same `crates/.../*.rs`,
   the same story file, or the same planning doc conflict. SEQUENCE them, or
   carve out non-overlapping regions and document the carve-out in both
   briefs (targeted-edit collision rule: two agents may share a file ONLY
   with explicit, disjoint line-range/section carve-outs in both briefs —
   whole-file rewrites never share).
2. **Touches the same module's public API?** If A introduces `pub fn foo()`
   and B imports `bar::*`, B will rebase. Spawn A first.
3. **Same `Cargo.toml`?** Two agents adding deps/features/[[test]] entries to
   one Cargo.toml conflict. SEQUENCE.
4. **Same generated artifact?** Anchored reports, insta/render snapshots,
   `sprint-status.yaml`, `trace.toml`, `CHANGELOG.md` — two agents both
   regenerating these collide. SEQUENCE (the triad files especially:
   one triad move per wave).
5. **Same operator-decide question?** If two agents need the SAME answer
   first, ask the operator before spawning either.

All-NO → safe to run concurrently, spawned in a single multi-Agent block.
Any YES → sequential waves. A 7-item agenda commonly maps to 1–3 waves of
3–5 agents; "spawn, wait, spawn, wait" with no conflict is an anti-pattern.

### Caveats that survive from experience

- **Default to sequential dev → ux when in doubt** — parallel sub-agents have
  no view of each other's reasoning (the chart-canvas-overhaul M7 collision).
  When the orchestrator can't articulate the lane split explicitly in the
  spawn briefs, don't parallelize.
- Story-file writes are single-writer: the seam that owns the story's current
  workflow step is the only writer of that story file in the wave.

## Decision framing — durable over quick (operator preference)

**The operator prefers one correct ship over two quick-and-incomplete ships.**
Reworking shipped features is expensive (context swap, anchor re-migration,
fresh review cycles); shipping the right thing once is cheaper. Every seam
MUST reflect this:

- Multi-option briefs put the `(Recommended)` tag on the **most durable**
  choice, NOT the cheapest — and surface the tradeoff explicitly ("costs +1
  week vs option B but avoids the v0.2.0 cleanup story"). Cost-frame as
  "rework risk over 6 months", not wall-clock days.
- A design that spawns "MIGRATION: remove at v0.2.0" comments is a yellow
  flag — reconsider the scope before building.
- Review verdicts name the rework cost of any carve-out explicitly; decks own
  the debt commitment alongside the ship.
- Orchestrator option curation: "quick win" options exist only when provably
  rework-free; default phrasing otherwise is "cheap fallback if budget
  tightens — adds a vN+1 cleanup commitment".

(Surfaced by the operator 2026-05-28 after repeatedly overriding
cheapest-option defaults.)

## Continuous work — don't pause unnecessarily

**Do not artificially throttle after each ship.** After a commit lands, the
default is to spawn the next unblocked workflow (create-story after planning;
dev-story after story-ready; code-review after dev-done), not to ask "what's
next?" — and NEVER to offer "call it a day" as an option. The operator stops
the session explicitly when they want to.

- When one agent is in flight and other work has independent file-scope,
  spawn it — and even when the linear chain is blocked, surface 3–4
  parallel-safe options (audits, hygiene sweeps, read-only spec-auditor
  passes) rather than one-track waiting.
- Status shape: "spawning X now; will report when it lands" — not "want me
  to spawn X or hold?".

## Session pre-flight

Run at session start (all exit 0 = silent/clean):

```bash
python3 scripts/queue_staleness_check.py     # backlog Queue rows vs story/board reality
python3 scripts/operator_ledger_check.py     # pending operator-side recipes (schema + stale-FAILED escalation)
```

Before promoting any backlog Queue entry to active work, verify the claim
against the board: `sprint-status.yaml` + the story file's `Status:` line
(and `git log` when in doubt) — Queue text lags reality; a stale row is
reconciled in `_bmad-output/planning-artifacts/backlog.md`, not built. The
board/story/trace state is a **hypothesis to verify against code and tests**,
not a fact (multiple "pending" items have turned out already-built).

The single source of truth for operator-run recipes that survive session
boundaries is
[`docs/dev-notes/operator-side-pending-ledger.md`](docs/dev-notes/operator-side-pending-ledger.md)
(orchestrator-maintained, append-only; schema enforced by the ledger check
above — FAILED rows older than 7 days escalate).

## Skills catalog

| Skill | Purpose |
|---|---|
| `bmad-*` (~47) | The BMAD workflow + persona set (see § The BMAD cycle). Customize ONLY via `_bmad/custom/` overrides — never edit the skill dirs. |
| `rust-build` | `cargo check` / `build` pipeline |
| `rust-test` | Full test matrix + report generation (template = the test-output contract) |
| `rust-validate` | fmt, clippy, audit, deny, docs |
| `rust-bench` | Criterion benchmarks with baseline diffs |
| `rust-coverage` / `rust-mutants` | Non-gating coverage / mutation reports (operator punch-list) |
| `backtest` | Historical strategy simulation |
| `verify-anchors` | Regression-gate the 119 body-SHA anchors in `evidence/anchors.toml` |
| `spec-lint` | Re-founded structural lint: stories/board/trace/CHANGELOG triad + dead links + anchors |
| `spec-brief` | Curated per-story context pack for delegation prompts (repointed per D5) |
| `present-results` | Assemble an operator deck (tech-writer seam); pre-tick gate `check_presentation.sh` |
| `capture-screenshot` | Capture (or operator-instruct) a UI screenshot |
| `cockpit-smoke` | **Orchestrator-only** pre-tick gate: boots fixtures cockpit + greps stderr for first-frame panics |
| `spec-update` | **RETIRED** (ratified D5, Phase 5c) — use the BMAD write-paths |

## Tooling — `scripts/`

| Script | Purpose | Caller |
|---|---|---|
| `scripts/verify_anchors.sh` | Verify all **119** anchors in `evidence/anchors.toml` | every gate run (mandatory) |
| `scripts/spec_lint.py` | Re-founded lint over `docs/` + `evidence/` + `_bmad-output/` (triad, dead links, orphan stories, anchors/trace); `--self-test` proves the rules | review seam, spec-auditor, orchestrator |
| `scripts/hash_report.py` | Body-only SHA-256 of a YAML-front-mattered report | review seam, dev |
| `scripts/adr_registry_check.py` | AD-18 atomicity lint over `planning-artifacts/architecture/decisions/` | architect seam, pre-commit |
| `scripts/queue_staleness_check.py` | Backlog Queue rows vs board reality | orchestrator pre-flight |
| `scripts/operator_ledger_check.py` | Operator-side pending-ledger schema + escalation | orchestrator pre-flight |
| `scripts/spec_brief.py` | Generate a per-story curated brief for delegation prompts | orchestrator |
| `scripts/check_presentation.sh` | Pre-tick guard: approval boxes un-ticked in a deck | tech-writer seam |
| `scripts/capture_screenshot.sh` | Darwin `screencapture` wrapper | orchestrator, tech-writer |
| `scripts/pre_stage_anchors.sh` | Stage candidate anchor SHAs from a backtest run | review seam (anchor refresh) |
| `scripts/prune_backtest_duplicates.sh` | Collapse duplicate backtest reports in `evidence/*/reports/` | review seam |
| `scripts/check_no_secrets_in_llm_artifacts.sh` | Guard: LLM artifacts contain no secrets | review seam, regression gate |
| `scripts/check_no_clocks_in_ui_tests.sh` | Guard: UI tests have no wall-clock dependency | review seam, regression gate |
| `scripts/check_no_raw_asof_join.sh` | Guard: no hand-rolled as-of join outside `core::pit` (ADR-0086) | dev seam pre-test, regression gate |
| `scripts/check_determinism_anchors.py` | AD-17 determinism spot-check against `evidence/anchors.toml` | review seam |

For the orchestrator-only `scripts/orch_*` set (cursor automation,
screencapture, cockpit on/off, supplement-log, determinism check, TCC probe,
PNG crop), see
[`docs/dev-notes/archive/2026-Q2/orchestrator-tooling-2026-05-12.md`](docs/dev-notes/archive/2026-Q2/orchestrator-tooling-2026-05-12.md).
Sub-agents must NOT call those — they wrap capabilities scoped to the
orchestrator's lane.

`evidence/anchors.toml` is the single source of truth for locked anchor SHAs
— never duplicate hashes into story/report files. Update only via architect
approval; the review seam locks new entries.

## Process discipline (lessons we paid for)

1. **Honest tick.** No task/AC is marked `[x]` without citing (a) the
   file:line where the change landed, (b) the test command exercising it,
   (c) the test-output line proving it passed. Can't cite all three → leave
   it unticked and hand off for verification.
2. **The reviewer owns final ticks.** The dev seam never flips a story past
   `review`; only the review seam does, after `VERDICT → PASS` AND
   `verify-anchors` PASS. Flipping to `done` is the triad move (story
   `Status:` + `trace.toml` `state=` + CHANGELOG line — `spec_lint` enforces).
3. **Anchor gate.** Any run that touched `crates/strategy/`, `crates/audit/`,
   `crates/exec/`, `crates/backtest/`, or report rendering MUST run
   `verify-anchors`. A single FAIL routes back to the dev seam with the body
   diff. The **119** anchors live in `evidence/anchors.toml`; nowhere else.
4. **Body-vs-front-matter discipline.** Anything that may differ between two
   equivalent runs (timestamps, wall-clock, host, pid, git commit,
   generated:, data_source variants) belongs in YAML front-matter — never in
   the body. The body is what gets hashed. *Why:* HF-1 (`wall_clock_s`) and
   T715 (`data_source` string) each cost a round.
5. **Determinism non-negotiables** (dev checklist):
   - No `SystemTime::now()` / `Instant::now()` reachable from a backtest
     replay path. Inject a clock.
   - No `f64` in money math. `rust_decimal::Decimal` + `Money<C>` newtype only.
   - Microsecond fractional-second timestamps in the audit DB — `Rfc3339`
     second-precision causes SQLite ORDER BY ties.
   - All RNGs `ChaCha20Rng::from_seed(...)`. No `thread_rng`.
   - HashMap iteration sorted before any cross-run comparison.
6. **UI pre-tick gate — `cockpit-smoke`.** Every UI story's review
   `VERDICT → PASS` triggers the
   [`cockpit-smoke`](.claude/skills/cockpit-smoke/SKILL.md) skill before the
   deck/approval step. Exit `0` → continue; exit `1` → block and route back
   to the dev seam with the panic-grep output. Cadence is **always-on**
   (operator-ratified): any story whose review cites `cargo test -p ui` runs
   it. **Orchestrator-only** per `## Capability boundaries` (needs a live
   window). *Why:* the F1 first-frame `fill_quad`/`unreachable!()` panic
   shipped past a 267-test snapshot suite because no harness exercised the
   live render path.
7. **Structural pre-tick gate — `spec-lint`.** Every review run AND every
   deck pre-tick MUST end with a clean
   [`spec-lint`](.claude/skills/spec-lint/SKILL.md)
   (`python3 scripts/spec_lint.py`, Python ≥ 3.11 — or `uv run`). Exit `0` →
   continue. Non-zero → block the verdict and route: content/link violations
   → the seam that owns the artifact; source-path violations
   (`trace-broken-path`) → the dev seam. Pairs with `verify-anchors`
   (content hashes) to give two mechanical gates: shape stability here,
   content stability there.

## Guardrails

- **Never** let any seam silently diverge from
  `_bmad-output/planning-artifacts/architecture.md` (the AD-1..AD-19 spine).
  Drift is either a spine/ADR update or a handoff — never both missing.
- **Never** accept a review/test report with missing sections; reject and re-run.
- **Never** ship on a REGRESSION verdict without a human "proceed anyway".
- **Never** use `unsafe` Rust without a `// SAFETY:` comment.
- **Never** commit secrets; keys live in env vars or the secret store defined
  in the architecture spine.
- **Never** create a git worktree, feature branch, or `claude/<slug>` branch.
  All work commits directly to `main` (see `## Branch & worktree policy`).
- **Never** let a sub-agent run `git commit` or `git push`. Sub-agents write
  files; the orchestrator commits + pushes.
- **Never** edit the `bmad-*` skill directories or `_bmad/` internals to
  change behavior — customization goes through `_bmad/custom/*.toml`
  (`bmad-customize`); BMAD upgrades must stay clean.

## Capability boundaries (orchestrator vs. sub-agent)

Adopted 2026-05-12 after the chart-canvas-overhaul retrospective
([ui-testing-direction-2026-05-12.md](docs/dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md)).
Sub-agents are **context tools, not capability tools**. Their toolset is a
subset of the orchestrator's. When a sub-agent's sandbox blocks a capability
the orchestrator has, the sub-agent must escalate, not rationalize.

| Capability | Owner | Sub-agents allowed? |
|---|---|---|
| `cargo fmt`, `cargo clippy`, `cargo test` (pure Rust) | sub-agent | yes |
| `verify_anchors.sh` / `spec_lint.py` / the other `scripts/` lints | sub-agent | yes |
| `rust-build`, `rust-validate`, `backtest` skills | sub-agent | yes |
| Story/planning-artifact writes under `_bmad-output/` | sub-agent | yes |
| `cargo run --bin cockpit` with a live window (`cockpit-smoke`) | **orchestrator** | **no** |
| `screencapture` of the running app | **orchestrator** | **no** |
| `osascript`, `cliclick`, Swift `CGWarp` cursor automation | **orchestrator** | **no** |
| Concluding "the bug is X" from live-app instrumentation | **orchestrator** | **no** |
| Adjudicating disagreements between sibling sub-agents | **orchestrator** | **no** |
| `git commit` / `git push` | **orchestrator** | **no** |
| Visual approval / rejection of UI; ticking an approval box | **operator** | **no** |

### Review split (fresh-context grading)

The old test-runner/evaluator split survives as the BMAD shape: the **dev
seam executes** gates and dumps raw output (no verdict), and
**`bmad-code-review` grades** in a fresh context that never saw the diff
being authored (different LLM recommended). `VERDICT → PASS/FAIL/REGRESSION`
emits from the review side only — this breaks the "agents skew positive when
grading their own work" failure mode.

### Architect = hypothesis only

Architect-seam output is **hypotheses with explicit falsifiers** ("if X,
measurement Y shows Z"). It does NOT run display-server/GPU instrumentation
or conclude "the bug is X" without citing an orchestrator-run empirical test
that refused to falsify. (The "iced has a half-scale canvas bug" misdiagnosis
cost 1.5 dev-days; this rule prevents the recurrence.)

## When NOT to use sub-agents

- Trivial one-file edits where spinning up an agent costs more than it saves.
- Purely conversational questions about the code.
- Quick compile checks after a one-line change.
- **Any task that requires a display server, GPU, screenshot, or window
  automation.** Per the capability map, the orchestrator owns these.

Use direct tools for those; reserve agents for work big enough to justify the
handoff overhead.
