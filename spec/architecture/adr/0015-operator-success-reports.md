---
adr: 0015
title: v1+ — Operator success reports architectural resolutions (Q1–Q9)
status: accepted
date: 2026-05-01
supersedes: none
superseded-by: none
---

# ADR-0015: v1+ — Operator success reports architectural resolutions (Q1–Q9)

## Context

The operator success report is a markdown deliverable (plus CSV
companion artefacts) summarising trading activity for a fixed window.
Nine architect-questions covered crate placement, audit-DB query
shape, file-write atomicity, sparkline rendering, CSV vs Parquet,
reconciliation tolerance, front-matter schema, kill-switch-trip
provenance, and the reflection-memory placeholder. All decisions
preserve the determinism contract that makes the
`report-sample-7d` / `report-sample-90d` anchors body-stable.

## Decisions

### Q1 — Crate placement: dedicated `crates/reports/`

New top-level workspace member `crates/reports/` (lib + bin
`report`). Cron-friendly and on-kill-switch invocable. Read-only over
`audit`, `data`, `cost`. No reverse edges. Avoids forcing the
`agent` bin into a reports role.

### Q2 — `pnl_by_strategy` reader + migration `004`

New `audit::query::pnl_by_strategy(ledger, since, until) ->
Vec<PnlBySymbolStrategyView>`. Schema migration
`004_journal_transactions_strategy_id.sql` adds a nullable
`strategy_id TEXT` column to `journal_transactions` plus a
`journal_transactions_sid_idx` index. Pre-migration rows have
`strategy_id = NULL` (legacy `equity:opening_balance` memo rows).

### Q3 — Atomic write: tempfile + `rename`

Writers emit to `<output>.tmp.<pid>`, `fsync_all`, then `rename` to
the final path. Crash-safe; readers never see a half-written file.
The same pattern applies to the CSV companions (Q5).

### Q4 — Sparkline format: Unicode block `▁▂▃▄▅▆▇█`

Eight-level Unicode-block palette (U+2581..U+2588) for inline
sparklines in the markdown. No external image rendering, no
PNG/SVG pipeline. The body remains byte-identical-able across runs
because the encoding is fully deterministic from the input series.

### Q5 — CSV vs Parquet: CSV companion artefacts

Companion CSVs alongside the markdown report, written via the same
atomic tempfile + rename pattern. Parquet deferred — operators
inspect with Excel / Numbers more than they query columnar; CSV is
the lowest-friction option until that changes.

### Q6 — Reconciliation tolerance: exact cent

`Decimal == Decimal` exact equality. No bps tolerance. Aligns with
[ADR-0003](0003-decimal-money-math.md) — money math is exact. A
non-zero ghost balance is always a bug to investigate, not a
threshold to tune.

### Q7 — Front-matter schema: 12 fixed fields

The analyst's 9-field set plus four (`agent_pid`, `host`,
`git_commit`, `run_id`) totaling 12. All run-varying fields stay
in front-matter (excluded from body-SHA per
[ADR-0004](0004-fractional-second-timestamps.md)). The body remains
byte-stable; the 12-field shape is locked.

### Q8 — Kill-switch-trip provenance: new `StrategyEventKind::KillSwitchTripped`

Add a `KillSwitchTripped` variant to the `strategy_events` table
(see [ADR-0008](0008-v05-strategy-event-journal-schema.md)). No
schema migration — `kind` is `TEXT`. Carries the trip reason
(`HaltReason::*`) in `error_code` and a short human summary in
`error_summary`. The reports binary reads it via
`strategy_events_since` and surfaces in the operator's "Mode &
session" section.

### Q9 — R6 reflection-memory placeholder lifecycle

v1+ reports ship R6 as a fixed placeholder string ("reflection
memory not yet implemented") in the markdown. The placeholder is
load-bearing because removing it later would break the body-SHA
anchor. Decision: when reflection-memory ships, the anchor is
re-locked once — not patched in place. The placeholder string is
part of the v1+ anchor body.

## Alternatives considered

- **Reports live in the `agent` bin.** Conflates orchestration with
  read-only reporting; complicates cron scheduling. Rejected.
- **`pnl_by_strategy` via JOIN at read time without the column.**
  Slow at scale; defeats the point of the migration. Rejected.
- **Parquet at v1+.** Higher friction than CSV for operator
  inspection. Deferred.
- **Front-matter tolerance for `host` / `pid`.** Defeats the
  body-stability invariant. Rejected — those fields stay in
  front-matter where they're excluded from the hash.
- **PNG / SVG sparklines.** Adds image-rendering dependency and
  breaks byte-stability. Rejected in favour of Unicode blocks.

## Consequences

- Two new body-SHA anchors lock at v1+ ship: `report-sample-7d`
  and `report-sample-90d`. Both depend on the placeholder-string
  contract from Q9.
- The pattern of writing report artefacts via tempfile + rename
  is now the project default for any non-append-only file output.
- `strategy_events.kind` grew another variant; the
  open-set-`TEXT`-column choice from
  [ADR-0008](0008-v05-strategy-event-journal-schema.md) continues
  to absorb new event types without schema migration.

## Changelog
- 2026-05-01 (architect): initial accept.
- 2026-05-13 (architect): extracted from `spec/architecture.md` §
  v1+ — Operator success reports resolutions during Phase 1A
  Session 9.
