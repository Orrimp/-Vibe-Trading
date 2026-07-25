# researcher — charter note (no v6 twin)

> Written during BMAD-migration Phase 5a
> (docs/dev-notes/bmad-migration-plan-2026-07-24.md § 6). This is a
> **preservation note, not a functional override** — there is no
> `bmad-agent-researcher` skill directory for `resolve_customization.py`
> to merge this against, so nothing reads this file automatically.
>
> **Disposition since Phase 5c (2026-07-25):** this charter is the live
> definition (the retired agent file is archived verbatim at
> `docs/archive/pre-bmad-agents/researcher.md`), and effectively **dormant**.
> The `research/` knowledge base is complete — 900/900 papers, 100 per
> topic across the 9 topic folders (`research/{backtesting,crypto-market-
> structure,data,deep-learning,evolution,llms,ml-trading,risk-and-sizing,
> strategies}/papers.md`, verified by count during this Phase-5a pass).
> This agent has no open work queued; this note exists for completeness
> and for the unlikely case the operator opens a 10th topic or a
> re-review pass.

## Why no v6 twin (and why that's low-stakes)

The closest BMAD mapping is `bmad-agent-analyst` +
`bmad-{domain,market,technical}-research` — the analyst persona already
absorbed the CURRENT-decision research workflows (market/domain/technical
research menu items CB/WB/MR/DR/TR live on `bmad-agent-analyst` by
default). What has no BMAD equivalent is specifically this agent's
**multi-day, resumable, one-topic-per-instance literature-review harvest**
shape — a different cadence and durability contract than analyst's
per-decision research calls.

## Condensed charter (full source archived: `docs/archive/pre-bmad-agents/researcher.md`)

- **Scope discipline:** owns exactly ONE topic folder under `research/<topic>/`
  and writes ONLY there — never another topic, never `PROGRESS.md`, never
  `spec/`/`crates/`/app code. A librarian, not a developer.
- **Resumable loop:** read the existing `papers.md` ledger first (skip
  duplicates, continue numbering from the last `[N]`); search via
  `WebSearch` preferring open access (arXiv/ar5iv/papers-with-code/SSRN
  abstracts); `WebFetch` each paper and log its entry IMMEDIATELY
  (before the next paper, so a crash loses at most one paper); aggregate
  into `knowledge.md` every ~5 papers.
- **Target:** 100 papers per topic across resumable rounds (~25-40 papers
  per invocation before context limits).
- **Ledger entry format:** Title / Authors-Venue / Year / Source / % read /
  3-6 sentence summary / relevance-to-our-system-or-"background only".
- **`knowledge.md` shape:** Key themes / Methods that hold up (and don't) /
  Actionable takeaways / Open questions worth testing / Paper map
  (claim -> supporting [N]).
- **Hard rule:** never fabricate a paper or a `% read` value — "15 real
  beats 25 invented." Does not commit (the orchestrator commits).

## What a future re-instantiation would need to preserve

1. The single-topic-folder write boundary — the mechanism that lets
   multiple researcher instances run in parallel without collision.
2. The append-after-every-paper resumability contract — without it a
   crash mid-run loses unbounded work instead of at most one paper.
3. The "real beats invented" anti-fabrication rule stated as a hard,
   non-negotiable constraint, not a soft preference.

No re-instantiation is currently planned — the knowledge base is complete
and the product has been in maintenance mode since 2026-07-09 (no
add-more-features roadmap; see `docs/dev-notes/do-not-build-register.md`
and the post-v2-scoping dev-note). If a 10th research topic is ever
opened, the lowest-friction path is a project-custom `bmad-research-
harvest` workflow skill scoped identically to the condensed charter
above.
