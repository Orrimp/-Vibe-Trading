---
name: spec-update
description: "DEPRECATED (retiring at BMAD-migration Phase 5c — see header below): safely update spec-driven development files (product, architecture, features, tasks, reports). Enforces frontmatter, keeps a changelog stub, and prevents accidental overwrites. Every agent MUST use this skill for spec writes instead of raw Write/Edit."
---

# spec-update

> **DEPRECATED — retiring at Phase 5c of the BMAD-METHOD v6.10.0 migration.**
> Ratified decision D5 (`docs/dev-notes/bmad-migration-plan-2026-07-24.md`
> § 10; recorded in commit `5582a74`'s message: "D5 spec-update
> retires/spec-brief repoints") resolved this skill's fate opposite the
> plan document's own §10 recommendation: **`spec-update` retires**,
> `spec-brief` stays alive and repoints (see that skill). The BMAD
> write-path (`bmad-create-story`/`bmad-dev-story`'s Dev Agent Record
> updates, `bmad-sprint-status` for status sync, and direct writes into
> `_bmad-output/planning-artifacts/` + `_bmad-output/implementation-artifacts/`
> per each workflow's own contract) supersedes it.
>
> **Content below is KEPT LIVE, not deleted**, for the transition window:
> `spec/` remains the single writable home for lifecycle state
> (`product.md`, `architecture.md`, `feature.md`, `tasks.md`, `trace.toml`)
> until the Phase 5b cutover commit (AD-4's migration write-lock,
> `_bmad-output/planning-artifacts/architecture.md` § AD-4) — agents still
> call this skill for those writes until then. Do not extend this skill
> with new capabilities; route new-capability asks to the BMAD write-path
> instead. Retires alongside `.claude/agents/*.md` at Phase 5c.

Single entry point for mutating `spec/` files. Ensures a consistent shape across
agents so humans and future Claude sessions can trust the docs.

## Files owned

| Path                              | Owner agent            | Purpose                                |
|-----------------------------------|------------------------|----------------------------------------|
| `spec/product.md`                 | analyst                | Product requirements & constraints     |
| `spec/architecture.md`            | architect              | System design, module map, budgets     |
| `spec/<slug>/feature.md`         | analyst → architect → developer | Per-feature lifecycle doc      |
| `spec/<slug>/tasks.md`            | architect → developer  | Ordered task list with checkboxes      |
| `evidence/<slug>/reports/<report>.md`     | tester, analyst        | Immutable, dated reports — never edit  |

## Frontmatter contract

Every non-report spec file starts with:

```yaml
---
slug: <kebab-case>
status: draft | in-progress | shipped | deprecated
owner: analyst | architect | developer | tester
updated: <YYYY-MM-DD>
---
```

Reports additionally carry `run_id`, `commit`, and `verdict`.

## Procedure

1. **Read before write.** If the target file exists, Read it, preserve any content
   you did not intend to touch, and bump `updated:`.

2. **Never overwrite reports.** They are append-only history. If a tester needs
   to re-run, create a new file with a new timestamp.

3. **Keep a "Changelog" section at the bottom** of product.md, architecture.md,
   and feature files. Add a one-line entry per edit:
   `- 2026-04-17 (architect): switched storage to ClickHouse — see ADR-0003.`

4. **Cross-link.** When you reference another spec file, use a relative link:
   `[architecture](../architecture.md)`.

5. **Fail closed.** If you cannot determine the right owner or status, ask
   rather than guess.

## Feature file skeleton

Use this shape for new `spec/<slug>/feature.md`:

```markdown
---
slug: <slug>
status: draft
owner: analyst
updated: <date>
---

# <Feature title>

## Why
_analyst fills this_

## Requirements
_analyst fills this_

## Design
_architect fills this_

## Backtest Scenarios
_analyst + architect fill this using the backtest/scenario template_

## Implementation
_developer fills this_

## Verification
_tester links to reports here_

## Changelog
```

## Task file skeleton

```markdown
---
slug: <slug>
status: in-progress
owner: developer
updated: <date>
---

# Tasks — <feature>

- [ ] T1 — <task> — _acceptance: <one-line criterion>_
- [ ] T2 — <task>
- [ ] T3 — <task>

## Notes
```
