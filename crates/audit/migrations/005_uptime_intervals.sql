-- T806 — Agent uptime intervals (operator success reports R7.1).
--
-- Append-only table.  Each agent boot writes one row on startup
-- (`open_uptime_interval`); a background heartbeat task updates
-- `last_heartbeat_at` every 30s; graceful shutdown sets `stopped_at`.
--
-- The reports binary computes uptime in `[period_start, period_end]` as:
--   Σ (min(stopped_at_or_last_heartbeat, period_end)
--      − max(started_at, period_start))
-- clamped to `[0, period_length]`.
--
-- Carries no money columns — the reconciler ignores this table.

CREATE TABLE IF NOT EXISTS agent_uptime (
    boot_id           TEXT PRIMARY KEY,         -- UUID v4 generated at agent boot
    started_at        TEXT NOT NULL,            -- RFC-3339 microsecond ts at boot
    last_heartbeat_at TEXT NOT NULL,            -- updated every heartbeat tick
    stopped_at        TEXT                      -- NULL while the agent is running
);

CREATE INDEX IF NOT EXISTS agent_uptime_started_idx
    ON agent_uptime(started_at);
