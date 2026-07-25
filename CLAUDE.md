# CLAUDE.md — Project Configuration

This repository is a **Rust crypto trading agent** — shipped as **"The Honest
Advisor"** (single-coin paper/sim investment advisor) — developed through the
**BMAD-METHOD v6 story-driven workflow** (migrated 2026-07-25; plan:
[`docs/dev-notes/bmad-migration-plan-2026-07-24.md`](docs/dev-notes/bmad-migration-plan-2026-07-24.md)).
Before doing anything non-trivial, read [AGENT.md](AGENT.md) — the orchestration
contract around the BMAD cycle.

## Where to start (file precedence for AI agents)

1. **[README.md](README.md)** — human-facing project overview + quickstart + status snapshot. Read this first to get oriented.
2. **[CHANGELOG.md](CHANGELOG.md)** — canonical "what's been built" index: one line per implemented feature, grouped by subsystem/version (kept at repo root by ratified decision D3; it is the third leg of the ADR-0082 triad). The fastest way to learn current state. Per-feature narrative lives in `git log` (historical paths under `spec/` still work as history).
3. **CLAUDE.md** (this file) — coding rules + skills + non-negotiables.
4. **[AGENT.md](AGENT.md)** — BMAD orchestration contract (personas, workflow cycle, orchestrator duties, capability boundaries).
5. **BMAD planning artifacts** — `_bmad-output/planning-artifacts/`:
   - [`PRD.md`](_bmad-output/planning-artifacts/PRD.md) — what this project IS and ISN'T (was `spec/product.md`).
   - [`architecture.md`](_bmad-output/planning-artifacts/architecture.md) — the architecture spine: 19 binding invariants AD-1..AD-19.
   - [`architecture/decisions/`](_bmad-output/planning-artifacts/architecture/decisions/README.md) — the ADR corpus (86+ ADRs) + Registry table.
   - [`epics.md`](_bmad-output/planning-artifacts/epics.md) — the 7 epics grouping ~142 stories.
   - [`trace.toml`](_bmad-output/planning-artifacts/trace.toml) — the requirement ledger (`[[req]]` rows; machine-checked).
   - [`backlog.md`](_bmad-output/planning-artifacts/backlog.md) — forward-looking Queue only (shipped work lives in CHANGELOG.md).
6. **BMAD implementation artifacts** — `_bmad-output/implementation-artifacts/`:
   - [`sprint-status.yaml`](_bmad-output/implementation-artifacts/sprint-status.yaml) — the live board (epic/story statuses).
   - `{epic}-{story}-{slug}.md` — one story per feature; the story's `Status:` line is the lifecycle source of truth (AD-4, story-keyed since Phase 5b).
7. **Project knowledge** — `docs/` (dev-notes, runbooks, design system, [do-not-build register](docs/dev-notes/do-not-build-register.md)); **evidence corpus** — `evidence/` (byte-immutable reports + presentations, `anchors.toml`).

## TL;DR for Claude — the workflow

- The canonical delivery loop is the **BMAD v6 cycle**, one workflow per fresh
  chat/agent context:
  `bmad-sprint-status` → `bmad-create-story` → `bmad-dev-story` →
  `bmad-code-review` → (repeat per story) → `bmad-retrospective` per epic.
  Planning-side: `bmad-prd`, `bmad-architecture`, `bmad-create-epics-and-stories`,
  `bmad-sprint-planning`. **`bmad-help`** is the entry point when unsure which
  workflow applies.
- **Personas** are invoked via their skills — `bmad-agent-{analyst,architect,dev,
  ux-designer,tech-writer,pm}` — each customized for this project by the
  committed overrides in **`_bmad/custom/*.toml`** (project knowledge,
  non-negotiables, harness-skill menus). The six legacy `.claude/agents/*`
  definitions are **RETIRED** (2026-07-25, archived at
  `docs/archive/pre-bmad-agents/`); the personas + the 14 project harness
  skills below replace them. Mapping table + the tester/presenter deltas:
  [AGENT.md § Persona mapping](AGENT.md#persona-mapping-legacy--bmad).
- The **orchestrator** (main session) still owns git (commit/push), gate
  verification before commit, and the parallelism rules — see AGENT.md.
- Durable output goes into `_bmad-output/` (stories, sprint-status, planning
  docs), `docs/` (knowledge), or `evidence/` (reports) — the chat is a view,
  not a store. The `spec-update` skill is **retired** (ratified D5); write via
  the BMAD workflows' own write-paths.
- The test-report template is still the contract for test output:
  [.claude/skills/rust-test/templates/test-report.md](.claude/skills/rust-test/templates/test-report.md).

## Repository map

```
trading/
├── AGENT.md                  # BMAD orchestration contract — read first
├── CLAUDE.md                 # This file
├── CHANGELOG.md              # The "what's been built" index (triad leg; root by D3)
├── Cargo.toml                # Workspace root (17 crates)
├── crates/                   # Rust workspace (audit, backtest, core, data, ui, …)
├── evidence/                 # Byte-immutable corpus (moved out of spec/ 2026-07-25)
│   ├── anchors.toml          # 119 locked body-SHA-256 regression anchors (keyed by NAME)
│   └── {v1,v2,v3,…}/<slug>/  # reports/ (anchored, frozen) + presentations/ (frozen)
├── docs/                     # Project knowledge (BMAD project_knowledge root)
│   ├── dev-notes/            # Cross-cutting memos, bug-log, do-not-build register
│   ├── runbooks/             # Operational runbooks (+ artifacts/)
│   ├── design/               # Lumen design system; ui-design-principles.md
│   └── archive/              # FROZEN history: pre-bmad-spec/ (retired spec/ tree),
│                             #   pre-bmad-agents/ (retired .claude/agents/)
├── _bmad/                    # BMAD install: config, manifests, scripts
│   └── custom/               # Committed persona/workflow overrides + charters
│                             #   (spec-auditor / ui-debugger / researcher)
├── _bmad-output/
│   ├── planning-artifacts/   # PRD.md · architecture.md · architecture/decisions/
│   │                         #   · epics.md · trace.toml · backlog.md
│   └── implementation-artifacts/  # sprint-status.yaml + one story per feature
├── .claude/
│   └── skills/               # 14 project harness skills + ~47 bmad-* workflow skills
├── research/                 # 900-paper knowledge base (complete)
├── scripts/                  # Gates: verify_anchors.sh, spec_lint.py, adr_registry_check.py, …
└── vendor/iced_tiny_skia/    # Operator-locked long-term fork (see below)
```

## Language & toolchain

- **Rust** stable, edition 2024.
- Preferred crates: `tokio`, `tracing`, `serde`, `thiserror`, `anyhow`
  (bins only), `reqwest`, `clap`, `criterion`, `proptest`.
- ML/DL default: `candle` for prototyping, `tract` for ONNX serving — confirm
  in [architecture.md](_bmad-output/planning-artifacts/architecture.md) before locking in.
- LLM default: Anthropic SDK with prompt caching; other providers behind a
  trait.

## Coding rules

- `Result<T, E>` in library code; no `.unwrap()` outside tests.
- No `println!` in library code — use `tracing`.
- Every external I/O behind a trait so tests can fake it.
- `unsafe` requires a `// SAFETY:` comment.
- `cargo fmt` on save; `cargo clippy -- -D warnings` must pass.
- **Iced/cockpit UI: verify at the rendered-PIXEL layer** (the `iced_test::Emulator::screenshot`
  harnesses — `render_snapshots.rs`, `live_equity_render.rs`, `reports_populated_curve_render.rs`),
  exercising the *populated* state with a negative control — NOT unit tests, text-summary
  snapshots, or a no-panic boot. Read the rendered PNG; a passing proxy is not proof the screen
  draws. Full guide: [`docs/dev-notes/iced-ui-render-verification.md`](docs/dev-notes/iced-ui-render-verification.md).

## What to do when the user asks for a change

1. Decide: is it trivial (one-file, no design impact) or non-trivial?
2. Trivial → direct edit, run `rust-build` + `rust-validate` yourself.
3. Non-trivial → the BMAD cycle per [AGENT.md](AGENT.md): check
   `sprint-status.yaml` and the do-not-build register, create/refine the story
   (`bmad-create-story`), implement it (`bmad-dev-story` or the dev persona),
   close with `bmad-code-review` + the harness gates (anchors, spec-lint,
   rust-validate). The orchestrator independently re-runs gates before commit.
4. Keep the artifacts honest — if the code and the story/PRD/architecture
   diverge, fix one of them before finishing (story `Status:`, `trace.toml`
   state, and the CHANGELOG line move together — the lint enforces it).

## Skills

Project harness skills (ours, under `.claude/skills/`; humans reference them by name):

- [`rust-build`](.claude/skills/rust-build/SKILL.md) · [`rust-test`](.claude/skills/rust-test/SKILL.md) · [`rust-validate`](.claude/skills/rust-validate/SKILL.md) · [`rust-bench`](.claude/skills/rust-bench/SKILL.md) · [`rust-coverage`](.claude/skills/rust-coverage/SKILL.md) · [`rust-mutants`](.claude/skills/rust-mutants/SKILL.md)
- [`backtest`](.claude/skills/backtest/SKILL.md) · [`verify-anchors`](.claude/skills/verify-anchors/SKILL.md) · [`spec-lint`](.claude/skills/spec-lint/SKILL.md) · [`spec-brief`](.claude/skills/spec-brief/SKILL.md)
- [`cockpit-smoke`](.claude/skills/cockpit-smoke/SKILL.md) (orchestrator-only) · [`capture-screenshot`](.claude/skills/capture-screenshot/SKILL.md) · [`present-results`](.claude/skills/present-results/SKILL.md)
- [`spec-update`](.claude/skills/spec-update/SKILL.md) — **RETIRED** at Phase 5c per ratified decision D5; use the BMAD write-paths.

The ~47 `bmad-*` skills (workflows + personas) come from the BMAD install; do
not edit them in place — customizations go through `_bmad/custom/` overrides
(`bmad-customize`).

## Code navigation (optional)

For fast navigation of this 715-crate-file Rust tree — "who calls `X`", "blast radius
of changing `Y`", "relevant symbols + source for area `Z`" in one call instead of
grepping — the repo is indexable with [CodeGraph](docs/dev-notes/codegraph.md)
(`codegraph callers|impact|explore <symbol>`). It is a **dev/agent aid only**: not a
Cargo dependency, not part of the product/runtime, **zero** effect on builds, tests,
or the `verify_anchors` gate. The `.codegraph/` index is gitignored. Setup + the
**opt-in** MCP wiring are in [`docs/dev-notes/codegraph.md`](docs/dev-notes/codegraph.md).

## Vendored dependencies

- `vendor/iced_tiny_skia/` is a **long-term local fork** of
  `iced_tiny_skia 0.14.0` plus the upstream canvas-clip fix from
  iced master commit `76b32d4906` (Jan 28, 2026). Wired via
  `[patch.crates-io]` in the workspace `Cargo.toml`. **Operator-
  locked 2026-05-20** — no iced 0.14.x patch branch exists; no
  iced upgrade expected near-term. See
  [`docs/archive/pre-bmad-spec/v1/chart-fixture-line-clipping/feature.md`](docs/archive/pre-bmad-spec/v1/chart-fixture-line-clipping/feature.md)
  for the maintenance contract — any future iced bump MUST audit
  the `Transformation::scale(scale_factor) * group.transformation()`
  ordering before retiring the fork.
- Any change to `vendor/*` other than the documented patch is out
  of scope — those files are upstream source. Bug reports go
  upstream.

## Non-negotiables

Carried **verbatim-in-force** through the BMAD migration (paths updated only;
the architecture spine's AD-1..AD-19 bind every design — on conflict the ADR
corpus wins):

- No secrets in git. Keys in env / secret store per
  [architecture.md](_bmad-output/planning-artifacts/architecture.md) (AD-19).
- No shipping on a `REGRESSION` verdict without an explicit human override (AD-19).
- No silent divergence from
  [`_bmad-output/planning-artifacts/architecture.md`](_bmad-output/planning-artifacts/architecture.md)
  — drift is either a spine/ADR update or a handoff, never both missing (AD-18).
- **The FROZEN robustness gate is byte-frozen (AD-1).** `classify_verdict` /
  `verdict_bands` / `compute_robustness_flag` / `rank_candidates` are not edited
  by feature work; every credibility/analytics addition proves it does not change
  ranking via an identity test. New arms only ever mean "more candidates face the
  same bar."
- **Anchors 119/119, byte-identical, before AND after any change (AD-2).**
  `bash scripts/verify_anchors.sh` must print `ANCHORS PASS (119 / 119)` before
  and after ANY edit touching `evidence/` — anchors live in
  [`evidence/anchors.toml`](evidence/anchors.toml) and are keyed by scenario
  **NAME**, not filename (grepping for a filename gives a false "not anchored").
- **Anchored report files in `evidence/*/reports/` are byte-immutable.** Per ADR-0038 § D6
  anchor-additive contract, even mechanical link-fix edits mutate the body-SHA and
  break the regression gate. Documentation-link cleanup sweeps MUST exclude anchored
  report files OR invoke the ADR-0038 § D6.b wiring-bug-fix re-emission protocol
  (or its future § D6.c documentation-link-fix variant once codified). The
  `evidence/**/presentations/` decks moved there in Phase 5b are frozen history too.
- **Every strategy overlay or sizing-modifier ships with a baseline-equity-divergence
  end-to-end test from day 1.** Per the `v3-volatility-forecaster-noop-fix` 2026-05-22
  precedent (see [`docs/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`](docs/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md)),
  unit tests on the math layer + anchored backtest reports are NOT sufficient to catch
  a no-op overlay where `scale` is computed but never applied. The required gate is an
  e2e test that asserts the overlay's output equity diverges from the un-targeted
  baseline equity by ≥ 1 bp (or some testable epsilon) when the strategy decision
  variable is non-trivial. Pattern reference: [`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`](crates/strategy/tests/vol_targeting_overlay_end_to_end.rs). (AD-16.)
- **UI ships only on rendered-pixel proof (AD-10)** — see the Coding rules bullet
  above; a passing proxy (unit test, text snapshot, no-panic boot) is not proof
  the screen draws.
- **Money math is Decimal, never f64 (AD-9).** No `f64` in money math —
  `rust_decimal::Decimal` + the `Money<C>` newtype only; exact-cent
  reconciliation in the double-entry ledger.
- **The do-not-build register is binding; the thesis is era-qualified (AD-11).**
  Check [`docs/dev-notes/do-not-build-register.md`](docs/dev-notes/do-not-build-register.md)
  before proposing ANY feature — settled dead-ends must not be re-proposed. The
  ship-passive claim is scoped to the **current era (2023+)**: real,
  cost-annex-robust, gate-crowned active edges existed 2017-20 and decayed to
  ~zero by 2023+ (none DSR-certified post scorecard-fix). **NEVER state the
  universal form.**
- **The ADR-0082 triad, story-keyed since Phase 5b:** a story's `Status:` line ↔
  its `trace.toml` `[[req]]` `state=` ↔ its CHANGELOG index line move together.
  Flipping a story to `done` requires the trace row at a shipped-terminal state
  and the CHANGELOG line in the same pass — `scripts/spec_lint.py` enforces all
  three legs (`status-drift`, `story-done-trace-drift`,
  `story-done-changelog-missing`).
- **ADR registration is atomic (AD-18).** Every non-trivial decision is a
  numbered ADR under
  [`_bmad-output/planning-artifacts/architecture/decisions/`](_bmad-output/planning-artifacts/architecture/decisions/README.md)
  **plus** its Registry row in the same commit — enforced by
  `scripts/adr_registry_check.py`. Numbers are never reused.
