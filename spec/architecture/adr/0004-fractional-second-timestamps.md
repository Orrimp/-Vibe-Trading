---
adr: 0004
title: Audit-DB timestamps use 6-digit fractional-second format
status: accepted
date: 2026-04-18
supersedes: none
superseded-by: none
---

# ADR-0004: Audit-DB timestamps use 6-digit fractional-second format

## Context

The audit-DB journal stores fills, orders, strategy events, and risk
events in temporal order. SQLite's `ORDER BY ts` is stable for unique
values; for ties it falls back to insertion order, which is not
deterministic across `--release` runs with concurrent inserts.

Two production incidents shaped this rule:

- **HF-1 (2026-04-18)**: `wall_clock_s` (`f64` seconds since epoch)
  leaked into the audit body. Same-second fills tied; `ORDER BY ts`
  returned them in non-deterministic order; the report body diffed
  between otherwise-equivalent runs, breaking the 9-anchor regression
  gate. The fix that day moved `wall_clock_s` to YAML front-matter, but
  the deeper issue — second-precision timestamps on a 1ms tick engine —
  needed a structural answer.
- **T715**: a `data_source:` path that varied between dev machines
  similarly leaked into the body. Same lesson: anything run-varying must
  live in front-matter or be deterministic.

The structural fix is to widen the timestamp to a precision where
genuine ties are vanishingly rare, and to make the format consistent
across the writer (`crates/audit/src/journal.rs`) and every reader.

## Decision

All audit-DB timestamp columns use the format
`YYYY-MM-DDTHH:MM:SS.uuuuuu` — six-digit microsecond fractional part,
RFC3339-shape but with explicit 6-digit precision. Writers MUST emit
exactly six digits (zero-padded). Readers MUST tolerate inputs with
fewer digits for backward compatibility, but the canonical format is
six.

Concrete invocation: in Rust, `chrono::DateTime<Utc>::format(
"%Y-%m-%dT%H:%M:%S%.6f")`. NEVER `chrono::SecondsFormat::Secs` or the
default `to_rfc3339()` (which is whatever-precision-fits-the-value and
collapses trailing zeros).

The schema migration is `crates/audit/migrations/<NNN>_<...>.sql`. New
columns added after this ADR follow the rule from day one.

## Alternatives considered

- **Store timestamps as `i64` microseconds since epoch.** Most efficient,
  zero ambiguity, but loses human-readability in SQLite `SELECT`. We
  store the formatted text alongside; the binary value is in a sibling
  column where downstream tools need it. Hybrid is over-engineered.
  Rejected for the canonical column; available as a secondary index
  column.
- **Nanosecond precision (`%.9f`).** Overkill — exchange feeds come in
  at millisecond at best, and the audit-DB cost (storage, index size,
  string comparison) is not justified. Rejected.
- **Insertion-order discriminator column.** Workable but ties the
  audit-DB schema to a non-temporal field; complicates replication and
  cross-process aggregation. Rejected.

## Consequences

- Mechanical enforcement: every PR touching `crates/audit` runs the
  developer determinism checklist (`.claude/agents/developer.md`) which
  includes "audit-DB timestamps use 6-digit fractional-second format".
  Test coverage: `crates/audit/tests/journal.rs::test_microsecond_ts`.
- The tester's `verify-anchors` skill catches any regression at body-SHA
  level. HF-1 would now be detected within one test run.
- The body-vs-front-matter discipline table in `.claude/agents/developer.md`
  is authoritative on which fields go where. Any new run-varying field
  goes in front-matter (excluded from the hash).
- Migration discipline: new columns added by future audit-DB migrations
  use `TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))`
  — the `%f` modifier produces millisecond precision; pad to six in the
  application layer.

## Changelog
- 2026-04-18 (architect): initial accept after HF-1. Promoted to a
  cross-cutting invariant in CLAUDE.md and `spec/architecture.md`.
  Extracted to ADR during Phase 1A split (2026-05-13).
