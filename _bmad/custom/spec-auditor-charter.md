# spec-auditor — charter note (no v6 twin)

> Written during BMAD-migration Phase 5a
> (docs/dev-notes/bmad-migration-plan-2026-07-24.md § 6). This is a
> **preservation note, not a functional override** — there is no
> `bmad-agent-spec-auditor` (or any other) skill directory for
> `resolve_customization.py` to merge this against, so nothing reads this
> file automatically. It exists so the charter survives Phase 5c's
> `.claude/agents/*.md` retirement even though the migration plan has no
> clean BMAD-native slot for it today.
>
> **Disposition since Phase 5c (2026-07-25):** this charter is now the live
> definition. The retired agent file is archived verbatim at
> `docs/archive/pre-bmad-agents/spec-auditor.md`; the specialist keeps
> running exactly as that charter describes — read-only weekly audit,
> invoked by name, no BMAD wiring involved.

## Why no v6 twin

`bmad-sprint-status` covers *some* of this surface (risk callouts on a
sprint board) but explicitly not anchor/trace drift, orphan feature
folders, dead intra-spec links, decay markers, or soft cross-file
contradictions — see `_bmad/custom/bmad-sprint-status.toml` for the
cross-reference note pointing back here. spec-auditor's job **is** this
project's own machinery (`scripts/spec_lint.py`, `scripts/verify_anchors.sh`,
the trace/CHANGELOG/anchor triad) — BMAD has no equivalent concept to
delegate to.

## Condensed charter (full source archived: `docs/archive/pre-bmad-agents/spec-auditor.md`)

- **Read-only.** Never edits `spec/` (or, post-migration, the story/evidence/
  docs trees). Never changes `evidence/anchors.toml`. Never changes the
  architecture spine. Produces a punch-list, not a verdict — does not block
  any other agent.
- **Procedure:** run `scripts/spec_lint.py --all` first, capture per-category
  counts vs. the previous audit; inventory every feature record by status/
  updated/owner and flag staleness (`proposed`/`in-progress` >30 days
  untouched, `shipped` with no test report); surface orphans (folders
  missing required files, reports for features that no longer exist, crates
  unmentioned anywhere); grep for TODO/FIXME/TBD/???/XXX decay markers
  (>5 per feature = decay-heavy); a small LLM-judged soft-contradiction
  sweep across 3-5 cross-cutting topics; cross-reference `anchors.toml`
  scenarios against shipped strategies for coverage gaps.
- **Output:** one dated dev-note (`docs/dev-notes/audit-<date>.md` at the
  post-Phase-4 path) via the (soon-retiring) `spec-update` skill, following
  a fixed template: Headline / Mechanical lint table / Stale features /
  Orphans / Decay markers / Soft contradictions / Anchor coverage gaps /
  Recommended triage (P1/P2/P3) / Changelog.
- **Cadence:** weekly (Monday 09:00 local), plus on-demand when the operator
  suspects drift.
- **Routing of findings:** `dead-link`/`trace-broken-path` -> developer or
  analyst; `missing-frontmatter`/`orphan-feature` -> the file's owner agent;
  `shipped-no-tests` -> tester; `unreferenced-anchor` -> architect;
  `soft contradiction` -> analyst then architect; `decay markers > 5` ->
  owner agent. The operator decides which findings become tasks — the
  auditor never self-authorizes a fix.

## What a future re-instantiation would need to preserve

1. The read-only guarantee (it is the ONE agent in this project explicitly
   forbidden from writing anything but its own dated dev-note).
2. The mechanical-lint-first ordering (`spec_lint.py --all` before any
   LLM-judged pass) — keeps the audit's hard findings trustworthy even if
   the soft-contradiction sweep is wrong.
3. The routing table — it is what makes the audit actionable rather than a
   wall of text.

Two re-instantiation paths, either viable, operator to decide when this
becomes live work: (a) a project-custom `bmad-spec-auditor` workflow skill
built via `bmad-builder`, invoked by name like any other workflow; or
(b) fold the weekly-cadence procedure into a `bmad-agent-architect`
`activation_steps_append` entry that runs on a schedule outside the normal
persona-menu flow. Neither is built yet — this file is scoped to Phase 5a
only.
