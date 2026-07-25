# spec/v2 — the research-driven next phase

This folder holds **v2 features**: the next phase of the framework, scoped from
the completed 900-paper research program. It is intentionally empty of features
at creation (2026-06-28) — the analyst and architect populate it.

## Where v2 comes from

- **`research/APPLICATIONS.md`** — the analyst/architect entry point: 21 per-topic
  application docs (Summary · Possible solutions · Relevance · Advantages · Problems
  · Concrete next steps with codebase locations + P0/P1/P2 · Open questions) + the
  convergent-priorities map.
- **`research/SYNTHESIS.md`** — the cross-topic distillation + the exact P0 spec.
- **`research/<topic>/knowledge.md`** — per-topic detail (the `papers.md` ledgers are
  the underlying evidence; not required reading for scoping).

## v1 vs v2

- **`spec/v1/`** — the shipped product: ~119 implemented feature folders (the
  single-coin crypto advisor + engine), moved here 2026-06-28. v1 is an **archive**
  of completed work; its anchored reports remain byte-immutable and the 119/119
  anchor gate validates against them in place.
- **`spec/v2/`** (this folder) — new, research-driven work. Not-yet-built features
  that predate the reorg (e.g. `advisor-reflection-decision-loop`) stay at `spec/`
  root pending triage into v2 or the backlog.
- **`spec/` root** — cross-cutting infra (`product.md`, `architecture.md`,
  `backlog.md`, `trace.toml`, `anchors.toml`, `architecture/` ADRs, `design/`,
  `runbooks/`, `dev-notes/`).

## The unchanged goal

A **framework for trading with traceable and plausible trading** — the advisor's
differentiator is **measured honesty, not asserted alpha**. The research's #1
convergent output (surfaced #1 in 6 of 9 topics) is the **P0 selection-bias /
overfitting scorecard** (N_eff → Deflated-Sharpe → MinBTL → PBO), additive to the
FROZEN bake-off gate — the first candidate v2 feature.

## How features land here

Per [AGENT.md](../../AGENT.md): analyst → architect → developer ‖ ui-designer →
tester → presenter. Each v2 feature gets its own `spec/v2/<slug>/feature.md` (+
`tasks.md`, `reports/`) and a `trace.toml` REQ row, scored by the **same frozen
robustness gate + buy-and-hold benchmark** as every v1 feature.
