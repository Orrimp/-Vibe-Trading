-- Strategy lifecycle events (T508, Q1 resolution).
--
-- A dedicated table for operator/system events (load/swap/unload/reject).
-- These are NOT balance-carrying rows — the reconciler skips this table.
-- Sibling to journal_entries in the same SQLite DB.

CREATE TABLE IF NOT EXISTS strategy_events (
    id            TEXT PRIMARY KEY,       -- uuid v4
    ts            TEXT NOT NULL,          -- RFC3339 (replay clock for research/backtest, wall-clock for paper)
    kind          TEXT NOT NULL,          -- 'Load' | 'Swap' | 'Unload' | 'Reject'
    strategy_id   TEXT,                   -- nullable for Reject when id unparsable
    old_hash      TEXT,                   -- sha256 hex, 64 chars; present for Swap and Unload
    new_hash      TEXT,                   -- sha256 hex, 64 chars; present for Load and Swap
    source_path   TEXT,                   -- repo-relative path under config/strategies/
    operator      TEXT NOT NULL DEFAULT 'system',  -- 'system' in v0.5; 'user' reserved for future
    error_code    TEXT,                   -- Reject only
    error_summary TEXT                    -- Reject only, short human message
);

CREATE INDEX IF NOT EXISTS strategy_events_ts_idx  ON strategy_events(ts);
CREATE INDEX IF NOT EXISTS strategy_events_sid_idx ON strategy_events(strategy_id, ts);
