-- Migration 010 — training_events table for cockpit-training-control v0.1.0
-- (Q1 = (a), R4.1-R4.5, ADR-0034 § D2).
--
-- Pure additive — `CREATE TABLE IF NOT EXISTS` + three indexes — no
-- ALTER, no data backfill, no UPDATE on any pre-existing row. The
-- migration is byte-safe against the 19 anchored backtest reports
-- (15 originals + 4 `-realdata`); none of them touch this table. The
-- migration is idempotent on re-run (sqlx tracks the version; the IF
-- NOT EXISTS guards re-application against a hand-touched DB).
--
-- The companion writers live at:
--   - crates/audit/src/journal.rs::post_training_start
--   - crates/audit/src/journal.rs::post_training_epoch
--   - crates/audit/src/journal.rs::post_training_finish
--   - crates/audit/src/journal.rs::post_training_failed
-- And the readers at:
--   - crates/audit/src/query.rs::recent_training_events
--       (sibling of recent_signals / recent_fills_filtered; same
--        RFC3339 binding + half-open [since, until) window)
--   - crates/audit/src/query.rs::latest_training_run
--       (convenience reader for the panel status strip)
--   - crates/audit/src/query.rs::orphan_training_runs
--       (the boot-time orphan-detect query per ADR-0034 § D7)
--
-- Schema notes:
--
-- * `id TEXT PRIMARY KEY` — UUID v4, matches every other audit table.
--   Composite (run_id, epoch) was considered and rejected because
--   `kind='start'` / `kind='failed'` rows have NULL epoch.
--   See ADR-0034 § D2.
--
-- * `kind TEXT NOT NULL` carries the 4-variant tag
--   ('start' | 'epoch' | 'finish' | 'failed'). No CHECK constraint —
--   the writer functions are the type-safety boundary; SQLite TEXT
--   tolerates trash but the journal API never produces trash.
--
-- * `train_loss` / `val_loss` stored as TEXT (Decimal-as-TEXT contract
--   per ADR-0003). The journal writers bind via `format!("{val}")` on
--   f32; the readers parse via `<f32 as FromStr>::from_str(...)`.
--   Lossless round-trip is fine for the observability surface — these
--   values feed a plot, not a determinism comparison.
--
-- * `pid INTEGER` — captured at the `start` emission edge so the
--   orphan-detect reader can do `libc::kill(pid, 0)`-based liveness
--   checks without round-tripping through external state. NULL on
--   non-start rows. PID-reuse is a known false-positive surface; the
--   24h orphan window bounds it. See ADR-0034 § D7.
--
-- * `scenario TEXT NOT NULL` — train_tcn always knows its scenario
--   label (default is the literal "default"). Making this column
--   non-null avoids a permanent `Option<SmolStr>` in the value type.
--
-- * `model_revision TEXT` — populated only on `kind='finish'` rows
--   from `CheckpointMetadata.model_revision` (the canonical SHA per
--   ADR-0029). NULL on every other kind.
--
-- * `ts TEXT NOT NULL` — RFC3339 with 6-digit microsecond precision
--   per ADR-0004 (T715 incident invariant).
--
-- Anchor risk: zero by construction. The 19 backtest body-SHA-256
-- anchors are computed over committed report bodies; this migration
-- adds a new table with no rows on existing DBs and is never read by
-- the backtest binary.

CREATE TABLE IF NOT EXISTS training_events (
    id              TEXT PRIMARY KEY,
    ts              TEXT NOT NULL,
    run_id          TEXT NOT NULL,
    kind            TEXT NOT NULL,
    epoch           INTEGER,
    total_epochs    INTEGER,
    train_loss      TEXT,
    val_loss        TEXT,
    wall_clock_ms   INTEGER,
    model_revision  TEXT,
    scenario        TEXT NOT NULL,
    seed            INTEGER NOT NULL,
    pid             INTEGER,
    error_message   TEXT
);

CREATE INDEX IF NOT EXISTS training_events_ts_idx
    ON training_events(ts);
CREATE INDEX IF NOT EXISTS training_events_run_id_idx
    ON training_events(run_id, ts);
CREATE INDEX IF NOT EXISTS training_events_kind_idx
    ON training_events(kind, ts);
