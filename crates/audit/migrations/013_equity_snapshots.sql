-- Migration 013 — equity_snapshots table for live-equity-history-durable v0.1.0
-- (A1, A3, ADR-0052).
--
-- Pure additive — `CREATE TABLE IF NOT EXISTS` + two indexes — no
-- ALTER, no data backfill, no UPDATE on any pre-existing row. The
-- migration is byte-safe against the 19 anchored backtest reports
-- (the backtest binary instantiates the reconciler with `bus = None`
-- and never touches this table). The migration is idempotent on
-- re-run (sqlx tracks the version; the IF NOT EXISTS guards re-
-- application against a hand-touched DB).
--
-- The companion writer lives at:
--   - crates/audit/src/journal.rs::post_equity_snapshot
-- And the readers at:
--   - crates/audit/src/query.rs::equity_snapshot_tail
--       (LIMIT ≤ 2880, monotone bar_ts order — for boot hydration)
--   - crates/audit/src/query.rs::purge_old_equity_snapshots
--       (age/row-capped DELETE — retention / R7)
--
-- Schema notes (A3):
--
-- * `id TEXT PRIMARY KEY` — UUID v4, matches every other audit table.
--
-- * `ts TEXT NOT NULL` — the row's mint wallclock, RFC3339 with 6-digit
--   microsecond precision (ADR-0004 — T715 invariant). Used for
--   retention purge (`DELETE WHERE ts < cutoff`) and forensics.
--
-- * `bar_ts TEXT NOT NULL` — the bar's close timestamp (data/bar time,
--   the chart x-axis). RFC3339-micros. This is `PnlSnapshot::bar_ts`
--   (the historical data time the equity curve plots) — kept SEPARATE
--   from `as_of` per the two-timestamp contract (approach A,
--   2026-06-11; reverted I1 showed conflating them breaks the curve).
--   Stored NOT NULL because the persistence gate is only reached after
--   `after_bar_close` / the research loop, which always supply bar_ts.
--
-- * `as_of TEXT NOT NULL` — wallclock delivery timestamp of the
--   snapshot (`PnlSnapshot::as_of = Timestamp::now()`). RFC3339-micros.
--   The UI delivery guard keys on this field's monotonicity (A4).
--
-- * `total_equity TEXT NOT NULL` — Decimal-as-TEXT (ADR-0003).
-- * `cash TEXT NOT NULL`         — Decimal-as-TEXT (ADR-0003).
-- * `realized TEXT NOT NULL`     — Decimal-as-TEXT (ADR-0003).
-- * `unrealized TEXT NOT NULL`   — Decimal-as-TEXT (ADR-0003).
--   All four money columns stored as TEXT so Decimal precision is
--   preserved across SQLite round-trips (no float approximation).
--
-- * `mode TEXT NOT NULL` — `'paper'` or `'live'` (the persistence gate
--   is `mode != Research`, so only paper/live rows are ever written).
--   Stored for forensics / future filtering. No CHECK constraint —
--   the writer is the type-safety boundary.
--
-- Indexes:
-- * `ts` index — used by the retention purge DELETE.
-- * `bar_ts` index — used by the tail query ORDER BY / LIMIT.
--
-- Anchor risk: zero by construction. The 19 backtest body-SHA-256
-- anchors are computed over committed report bodies; this migration
-- adds a new table with no rows on existing DBs and is never read by
-- the backtest binary.

CREATE TABLE IF NOT EXISTS equity_snapshots (
    id           TEXT PRIMARY KEY,
    ts           TEXT NOT NULL,
    bar_ts       TEXT NOT NULL,
    as_of        TEXT NOT NULL,
    total_equity TEXT NOT NULL,
    cash         TEXT NOT NULL,
    realized     TEXT NOT NULL,
    unrealized   TEXT NOT NULL,
    mode         TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS equity_snapshots_ts_idx
    ON equity_snapshots(ts);
CREATE INDEX IF NOT EXISTS equity_snapshots_bar_ts_idx
    ON equity_snapshots(bar_ts);
