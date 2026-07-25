# Convention — encoding a "shipped-partial" status (2026-05-16)

Operator decision: keep the `spec-update` skill's status enum as-is
(`draft | in-progress | shipped | deprecated`) and encode partial-final
state via a small frontmatter pattern instead of a new token.

## When to use

A feature that landed but is intentionally terminal at less than its
full scope — typical example: `ui-gallery-bin` shipped V1–V4 but V5+
was blocked on an upstream bug and routed to a successor feature.

## Convention

```yaml
---
slug: <kebab-case>
status: shipped                       # use the canonical token
version: <X.Y.Z>-partial-terminal     # version-suffix carries the signal
owner: <agent>
updated: <date>
successor: <successor-slug> v<X.Y.Z>  # NEW field — points to the follow-up
---
```

And a body callout immediately under the H1 / TL;DR:

> **Status (YYYY-MM-DD):** vX.Y.Z-partial-terminal. <one-line description of
> what's covered>. V<N>+ tracked in successor `<successor-slug>` (`../<successor-slug>/feature.md`).

The matching successor feature.md carries the inverse:

```yaml
predecessor: <predecessor-slug> v<X.Y.Z>-partial-terminal
```

## Why not amend the enum?

- spec-lint and any other tooling that validates the status enum
  doesn't need to grow a new token — the canonical `shipped` is still
  honest (the work that landed did ship).
- The version-suffix + `successor:` field carries strictly more
  information than a single status token would: it names the follow-up.
- Reverting the convention later is cheap (delete the suffix + field).

## First applied at

- `spec/ui-gallery-bin/feature.md` — `version: 0.1.0-partial-terminal`,
  `successor: ui-gallery-table-cell`.
- `spec/ui-gallery-table-cell/feature.md` —
  `predecessor: ui-gallery-bin v0.1.0-partial-terminal`.

Both edits are in the [Wave 2a analyst report](feature-triage-2026-05-16.md)
trail.

## Future-work

If `shipped-partial` cases multiply (say, > 3 features at any time), revisit:
formalize as an enum token, update spec-update SKILL.md, teach spec-lint.
