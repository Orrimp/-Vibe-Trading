---
adr: 0024
title: Audit ledger uses raw `sqlx` against embedded SQLite, not `sqlx-ledger`
status: accepted
date: 2026-04-19
supersedes: none
superseded-by: none
---

# ADR-0024: Audit ledger uses raw `sqlx` against embedded SQLite, not `sqlx-ledger`

## Context

The audit goal in [`../product.md`](../../../../spec/product.md) requires every
trading decision, intent, order, fill, and P&L attribution to be
auditable. Double-entry bookkeeping is the right shape: balanced
debit/credit journal lines per transaction, append-only, with a
chart of accounts. Week 1's first pick was `sqlx-ledger` — a crate
that purports to provide exactly this. It didn't ship.

## Decision

The `audit` crate uses raw `sqlx` against an embedded SQLite file,
with a small set of in-repo schema migrations
(`crates/audit/migrations/`). We retain the **semantics** of
`sqlx-ledger` (double-entry, balanced-per-txn, append-only journal,
idempotent chart-of-accounts bootstrap) and drop only the
dependency.

Shape (actual code in `crates/audit/src/`):

- `ledger.rs` — `Ledger::open(db_path)` + `sqlx::migrate!` runs the
  in-repo migrations; `:memory:` path for tests.
- `bootstrap.rs` — `chart_of_accounts()` inserts the v0 accounts
  idempotently (`INSERT OR IGNORE`).
- `journal.rs` — `post_fill(&Fill)` writes one `journal_transactions`
  row plus N `journal_entries` rows inside a single `sqlx`
  transaction; buy / sell / fee legs balance to the satoshi.
  `registry_event()` and `kill_switch_tripped()` write zero-amount
  memo rows against `equity:opening_balance` to preserve the
  balance invariant. v0.5+ replaces zero-amount memo rows for
  strategy events with the dedicated `strategy_events` table; see
  [ADR-0008](0008-v05-strategy-event-journal-schema.md).
- `query.rs` — `cash_balance`, `realized_pnl_since`, `total_fees`,
  `account_list`, `recent_fills`, `recent_journal`,
  `all_transaction_ids`, `global_debit_credit_sum`. None return
  `sqlx` types — only `Decimal`, `Money<C>`, and `core` view types.

The backend (`sqlx::SqlitePool`) is crate-private; no consumer
imports `sqlx` types from `audit`'s public API.

## Why not `sqlx-ledger`

Week 1 wiring discovered that `sqlx-ledger v0.11.14` is
**Postgres-only** — its `Cargo.toml` gates the store behind
`sqlx/postgres`; no SQLite path compiles. Adopting it would have
forced Postgres as an ops dep and broken the single-binary deploy
goal locked in [`../product.md` § Project scope boundary](../../../../spec/product.md#project-scope-boundary).
A SQLite port would be a multi-week fork job, outside v0 budget.

## Alternatives considered

- **`sqlx-ledger` on SQLite.** Postgres-only as shipped; rejected at
  build time.
- **`cala-ledger` on Postgres.** Even more Postgres-locked; forces
  a DB process. Same reason, stronger.
- **Leave the substrate open.** Feature work needs a stable journal
  / query surface to build against; can't delay. Rejected.

## Consequences

- Single-binary deploy holds; zero ops cost; fits the `$20/month`
  hosting line in [`../product.md` § Cost economics](../../../../spec/product.md#cost-economics--monthly-ceiling).
- Embedded SQLite WAL handles the v0 write rate (≤ a few hundred
  journal entries per minute at 1m bars) trivially. Backup = copy
  the file (see [`08-recovery-and-backups.md`](../../../../spec/architecture/08-recovery-and-backups.md)).
- Future swap (Postgres-backed ledger, or a revived `sqlx-ledger`
  with SQLite support) is a one-file change inside `audit` because
  the public API exposes only `Decimal` / `Money<C>` / view types,
  never `sqlx` types. The Decimal-in / Decimal-out contract is the
  load-bearing constraint; the storage engine is not.

## Changelog
- 2026-04-19 (architect): initial accept (reconciled after the
  failed `sqlx-ledger` adoption).
- 2026-05-13 (architect): extracted from `spec/architecture.md` §
  Foundation libraries — Audit & ledger during Phase 1A Session 11.
