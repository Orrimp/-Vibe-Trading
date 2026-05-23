# CLAUDE.md — Project Configuration

This repository is a **Rust crypto trading agent** driven by a spec-driven,
multi-agent workflow. Before doing anything non-trivial, read [AGENT.md](AGENT.md).

## Where to start (file precedence for AI agents)

1. **[README.md](README.md)** — human-facing project overview + quickstart + status snapshot + feature groups. Read this first to get oriented.
2. **CLAUDE.md** (this file) — coding rules + skills + non-negotiables.
3. **[AGENT.md](AGENT.md)** — multi-agent orchestration contract (analyst → architect → developer ‖ ui-designer → tester → presenter).
4. **[spec/product.md](spec/product.md)** — what this project IS and ISN'T (analyst-owned).
5. **[spec/architecture.md](spec/architecture.md)** — system design (architect-owned).
6. **[spec/backlog.md](spec/backlog.md)** — Active / Queue / Recent.
7. **[spec/dev-notes/feature-state-table-2026-05-22.md](spec/dev-notes/feature-state-table-2026-05-22.md)** — 54-feature comprehensive inventory.

## TL;DR for Claude

- The canonical workflow is **analyst → architect → (developer ‖ ui-designer)
  → tester → presenter → human**, with bidirectional feedback routes.
  Developer and ui-designer run in parallel whenever a feature has both
  backend and UI surface. The presenter is the agile sprint-review face —
  it assembles a `spec/<slug>/presentations/<slug>-<date>.md` for operator approval
  and runs only after `VERDICT → PASS`.
- Use **sub-agents in parallel** whenever their work is independent — see
  [AGENT.md](AGENT.md#parallelism-rules).
- All durable output goes into `spec/` via the [`spec-update`](.claude/skills/spec-update/SKILL.md)
  skill — never write to `spec/` files directly with raw Write/Edit.
- The tester's report template is the contract for test output:
  [.claude/skills/rust-test/templates/test-report.md](.claude/skills/rust-test/templates/test-report.md).

## Repository map

```
trading/
├── AGENT.md                 # Orchestration & workflow — read first
├── CLAUDE.md                # This file
├── Cargo.toml               # Workspace root (will become virtual workspace)
├── src/                     # Temporary main.rs — will be split into crates/
├── .claude/
│   ├── agents/              # analyst, architect, developer, tester
│   └── skills/              # rust-build, rust-test, rust-validate,
│                            # rust-bench, backtest, spec-update
└── spec/
    ├── product.md           # Product requirements (analyst-owned)
    ├── architecture.md      # System design (architect-owned)
    ├── backlog.md           # Roadmap / queue
    ├── anchors.toml         # Locked body-SHA-256 regression anchors
    ├── ui-design-principles.md   # Cross-cutting UI codex
    ├── design/              # Lumen design system (cross-phase)
    ├── runbooks/            # Operational runbooks
    ├── archive/             # Compressed historical reports
    ├── dev-notes/           # Cross-cutting dev memos
    └── <feature-slug>/      # Per-feature folder
        ├── feature.md       # Brief (frontmatter has version: x.y.z)
        ├── tasks.md         # Task list
        ├── reports/         # test-*.md, backtest-*.md, screenshots/
        └── presentations/   # Operator decks + artifacts/
```

The crate layout under `crates/` is proposed in
[spec/architecture.md](spec/architecture.md) and will materialize as features
land.

## Language & toolchain

- **Rust** stable, edition 2024.
- Preferred crates: `tokio`, `tracing`, `serde`, `thiserror`, `anyhow`
  (bins only), `reqwest`, `clap`, `criterion`, `proptest`.
- ML/DL default: `candle` for prototyping, `tract` for ONNX serving — confirm
  in [architecture.md](spec/architecture.md) before locking in.
- LLM default: Anthropic SDK with prompt caching; other providers behind a
  trait.

## Coding rules

- `Result<T, E>` in library code; no `.unwrap()` outside tests.
- No `println!` in library code — use `tracing`.
- Every external I/O behind a trait so tests can fake it.
- `unsafe` requires a `// SAFETY:` comment.
- `cargo fmt` on save; `cargo clippy -- -D warnings` must pass.

## What to do when the user asks for a change

1. Decide: is it trivial (one-file, no design impact) or non-trivial?
2. Trivial → direct edit, run `rust-build` + `rust-validate` yourself.
3. Non-trivial → follow [AGENT.md](AGENT.md): analyst first, parallel
   sub-agents where independent, tester always closes the loop with a report.
4. Keep `spec/` honest — if the code and the spec diverge, fix one of them
   before finishing.

## Skills

Agents use these via the Skill tool; humans reference them by name:

- [`rust-build`](.claude/skills/rust-build/SKILL.md)
- [`rust-test`](.claude/skills/rust-test/SKILL.md)
- [`rust-validate`](.claude/skills/rust-validate/SKILL.md)
- [`rust-bench`](.claude/skills/rust-bench/SKILL.md)
- [`backtest`](.claude/skills/backtest/SKILL.md)
- [`spec-update`](.claude/skills/spec-update/SKILL.md)

## Vendored dependencies

- `vendor/iced_tiny_skia/` is a **long-term local fork** of
  `iced_tiny_skia 0.14.0` plus the upstream canvas-clip fix from
  iced master commit `76b32d4906` (Jan 28, 2026). Wired via
  `[patch.crates-io]` in the workspace `Cargo.toml`. **Operator-
  locked 2026-05-20** — no iced 0.14.x patch branch exists; no
  iced upgrade expected near-term. See
  [`spec/chart-fixture-line-clipping/feature.md`](spec/chart-fixture-line-clipping/feature.md)
  for the maintenance contract — any future iced bump MUST audit
  the `Transformation::scale(scale_factor) * group.transformation()`
  ordering before retiring the fork.
- Any change to `vendor/*` other than the documented patch is out
  of scope — those files are upstream source. Bug reports go
  upstream.

## Non-negotiables

- No secrets in git. Keys in env / secret store per architecture.md.
- No shipping on a `REGRESSION` verdict without an explicit human override.
- No silent divergence from `spec/architecture.md`.
- **Every strategy overlay or sizing-modifier ships with a baseline-equity-divergence
  end-to-end test from day 1.** Per the `v3-volatility-forecaster-noop-fix` 2026-05-22
  precedent (see [`spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`](spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md)),
  unit tests on the math layer + anchored backtest reports are NOT sufficient to catch
  a no-op overlay where `scale` is computed but never applied. The required gate is an
  e2e test that asserts the overlay's output equity diverges from the un-targeted
  baseline equity by ≥ 1 bp (or some testable epsilon) when the strategy decision
  variable is non-trivial. Pattern reference: [`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`](crates/strategy/tests/vol_targeting_overlay_end_to_end.rs).
- **Anchored report files in `spec/*/reports/` are byte-immutable.** Per ADR-0038 § D6
  anchor-additive contract, even mechanical link-fix edits mutate the body-SHA and
  break the regression gate. Documentation-link cleanup sweeps MUST exclude anchored
  report files OR invoke the ADR-0038 § D6.b wiring-bug-fix re-emission protocol
  (or its future § D6.c documentation-link-fix variant once codified).
