---
name: spec-brief
description: Assemble a small (~5k-token) per-feature briefing pack so sub-agents work from curated context rather than reading 296 KB of architecture.md. The orchestrator MUST invoke this before delegating any feature-scoped work to architect / developer / ui-designer. Read-only; never edits spec.
---

# spec-brief

Pre-flight context assembler. Pairs with `spec-update` (writes) and
`spec-lint` (shape checks) to form the spec-toolkit triad.

## Why this exists

`spec/architecture.md` is 296 KB / 5,635 lines. No agent can read it in one
turn. Without this skill, sub-agents either grep blindly or run out of
context. This skill produces a self-contained brief from:

1. CLAUDE.md non-negotiables (always verbatim).
2. The feature's `feature.md` and `tasks.md`.
3. Any `trace.toml` rows that reference the feature.
4. The most recent test report under `spec/<slug>/reports/`.
5. The full anchor table from `spec/anchors.toml` (small, always included).
6. Architecture-section excerpts that mention the slug (best-effort grep
   until `spec/architecture/*.md` lands per Phase 1A).

## Procedure

1. List available slugs (optional):

   ```bash
   scripts/spec_brief.py --list
   ```

2. Generate a brief to stdout:

   ```bash
   scripts/spec_brief.py <slug>
   ```

3. Generate to a file (for handoff to a sub-agent):

   ```bash
   scripts/spec_brief.py <slug> --out /tmp/brief-<slug>.md
   ```

4. The script prints the char count and rough token estimate to stderr.
   Treat anything above ~7,000 tokens as a smell — either the feature is
   too broad or the architecture mentions are too noisy.

## Routing

This skill is **invoked by the orchestrator**, not by sub-agents. The
orchestrator generates the brief, passes the path to the sub-agent in the
delegation prompt, and the sub-agent reads it first thing.

Sub-agents that should consume a brief:

- analyst (when refining an existing feature; not for greenfield)
- architect (always, for feature-scoped work)
- developer (always)
- ui-designer (always)
- tester (always; brief tells them which anchors to gate against)
- presenter (always; brief is the input to the deck assembly)

## When to invoke

Mandatory before any feature-scoped sub-agent run.

Optional but cheap:
- Before a code review to refresh context.
- After a long pause on a feature to re-orient.

## What this skill does NOT do

- Does not modify `spec/`.
- Does not invent content — it only assembles what already exists.
- Does not replace `spec-update`, which is still the only path for writes.
- Does not deduplicate against the orchestrator's own context — the brief is
  written for the sub-agent that has none.
