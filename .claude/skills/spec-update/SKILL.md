---
name: spec-update
description: "RETIRED (BMAD-migration Phase 5c, 2026-07-25 — ratified decision D5): the legacy safe-writer for the retired spec/ tree. Do NOT invoke. Durable writes now go through the BMAD workflows' own write-paths (stories + sprint-status under _bmad-output/implementation-artifacts/, planning docs under _bmad-output/planning-artifacts/, knowledge under docs/, new dated reports/decks under evidence/<slug>/)."
---

# spec-update — RETIRED

> **RETIRED at Phase 5c of the BMAD-METHOD v6.10.0 migration (2026-07-25).**
> Ratified decision D5 (`docs/dev-notes/bmad-migration-plan-2026-07-24.md`
> § 10; recorded in commit `5582a74`'s message: "D5 spec-update
> retires/spec-brief repoints"). The `spec/` tree this skill wrote to no
> longer exists — it was retired at Phase 5b (content now lives in
> `_bmad-output/` stories/planning docs, `docs/` knowledge, and the frozen
> archives under `docs/archive/pre-bmad-spec/`).
>
> This file is kept as a tombstone so stale references fail loudly and
> readably instead of silently. Do not extend it; do not invoke it.

## Where writes go now

| What you are writing | Where it goes |
|---|---|
| Per-feature lifecycle (story, `Status:`, Tasks/Subtasks, Dev Agent Record) | `_bmad-output/implementation-artifacts/{epic}-{story}-<slug>.md` via `bmad-create-story` / `bmad-dev-story` |
| Board state | `_bmad-output/implementation-artifacts/sprint-status.yaml` via `bmad-sprint-planning` / `bmad-sprint-status` |
| Product requirements | `_bmad-output/planning-artifacts/PRD.md` via `bmad-prd` |
| Architecture + ADRs | `_bmad-output/planning-artifacts/architecture.md` + `architecture/decisions/` (AD-18 atomic registration; `scripts/adr_registry_check.py`) |
| Requirement ledger | `_bmad-output/planning-artifacts/trace.toml` (machine-checked by `scripts/spec_lint.py`) |
| Forward queue | `_bmad-output/planning-artifacts/backlog.md` |
| Cross-cutting knowledge (dev-notes, runbooks, design) | `docs/` |
| Test/backtest reports, operator decks | NEW dated files under `evidence/<slug>/reports/` / `evidence/<slug>/presentations/` — never edit an existing one (byte-immutable once anchored; ADR-0038 § D6) |

Rules that OUTLIVE this skill (they were never really this skill's — they are
CLAUDE.md non-negotiables):

- Reports are append-only history; re-runs create a new timestamped file.
- Anchored bodies are byte-immutable; `bash scripts/verify_anchors.sh` must
  print 119/119 before and after any change near `evidence/`.
- Fail closed: if you cannot determine the right owner/home for a write, ask
  rather than guess.

The historical skill body (frontmatter contract, feature.md / tasks.md
skeletons) is preserved in git history —
`git log -- .claude/skills/spec-update/SKILL.md` — and the file shapes it
governed live on, frozen, under `docs/archive/pre-bmad-spec/`.
