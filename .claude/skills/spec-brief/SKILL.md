---
name: spec-brief
description: Assemble a small (~5k-token) per-feature briefing pack so sub-agents work from curated context rather than reading the full BMAD architecture spine. The orchestrator SHOULD invoke this before delegating feature-scoped work to the architect / dev / ux-designer seams. Read-only; never edits any artifact.
---

# spec-brief

Pre-flight context assembler (kept and repointed at the BMAD migration per
ratified decision D5; the old write-side sibling `spec-update` is retired).
Pairs with `spec-lint` (shape checks) in the toolkit.

## Why this exists

The architecture spine (`_bmad-output/planning-artifacts/architecture.md`,
~4,700 lines) plus the PRD, trace ledger, and evidence corpus are too large
for a sub-agent to read whole. Without this skill, sub-agents either grep
blindly or run out of context. `scripts/spec_brief.py` produces a
self-contained brief from:

1. CLAUDE.md non-negotiables (always verbatim).
2. The feature's **story** (`_bmad-output/implementation-artifacts/{epic}-{story}-<slug>.md`).
3. Any `_bmad-output/planning-artifacts/trace.toml` rows that reference the feature.
4. The most recent test report under `evidence/<slug>/reports/`.
5. The full anchor table from `evidence/anchors.toml` (small, always included).
6. Architecture-spine excerpts that mention the slug (bounded grep windows).

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
orchestrator generates the brief, passes the path in the delegation prompt,
and the sub-agent reads it first thing.

Seams that should consume a brief:

- analyst persona (when refining an existing feature; not for greenfield)
- architect persona (always, for feature-scoped work)
- dev persona / `bmad-dev-story` (always — complements the story's own
  embedded context)
- ux-designer persona (always)
- `bmad-code-review` (always; the brief names which anchors to gate against)
- tech-writer persona (deck assembly input)

## When to invoke

Recommended before any feature-scoped sub-agent run (BMAD's `bmad-create-story`
also embeds context into the story file itself; the brief supplements it with
the anchor table + non-negotiables + latest evidence).

Optional but cheap:
- Before a code review to refresh context.
- After a long pause on a feature to re-orient.

## What this skill does NOT do

- Does not modify anything. Read-only.
- Does not invent content — it only assembles what already exists.
- Does not deduplicate against the orchestrator's own context — the brief is
  written for the sub-agent that has none.
