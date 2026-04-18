---
name: spec-update
description: Safely update spec-driven development files (product, architecture, features, tasks, reports). Enforces frontmatter, keeps a changelog stub, and prevents accidental overwrites. Every agent MUST use this skill for spec writes instead of raw Write/Edit.
---

# spec-update

Single entry point for mutating `spec/` files. Ensures a consistent shape across
agents so humans and future Claude sessions can trust the docs.

## Files owned

| Path                              | Owner agent            | Purpose                                |
|-----------------------------------|------------------------|----------------------------------------|
| `spec/product.md`                 | analyst                | Product requirements & constraints     |
| `spec/architecture.md`            | architect              | System design, module map, budgets     |
| `spec/features/<slug>.md`         | analyst → architect → developer | Per-feature lifecycle doc      |
| `spec/tasks/<slug>.md`            | architect → developer  | Ordered task list with checkboxes      |
| `spec/reports/<report>.md`        | tester, analyst        | Immutable, dated reports — never edit  |

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

Use this shape for new `spec/features/<slug>.md`:

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
