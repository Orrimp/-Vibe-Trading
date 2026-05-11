-- Migration 009 — strategy_signals table for chart-buy-sell-emphasis v1.9
-- (Q1 = (a), R5.3, R5.7).
--
-- Pure additive — `CREATE TABLE IF NOT EXISTS` + three indexes — no
-- ALTER, no data backfill, no UPDATE on any pre-existing row. The
-- migration is byte-safe against the 11 anchored backtest reports
-- (none of them touch this table) and is idempotent on re-run (sqlx
-- tracks the version; the IF NOT EXISTS guards re-application against
-- a hand-touched DB).
--
-- The companion writers live at:
--   - crates/audit/src/journal.rs::post_strategy_signal
--       (one INSERT per emitted Signal; called from the agent main
--       loop's signal-eval tap point — a future agent-runtime
--       follow-up brief actually wires the call site)
--   - crates/audit/src/journal.rs::update_signal_clamp_status
--       (one UPDATE per risk-decision; flips `was_clamped` +
--       `clamp_reason` once the risk engine has consumed the signal)
-- And the reader at:
--   - crates/audit/src/query.rs::recent_signals
--       (sibling of recent_fills_filtered; same RFC3339 binding +
--       venue.to_string() + half-open [since, until) window)
--
-- The whole table is gated by `agent.toml [signal_log] enabled = false`
-- per architect Q1 resolution. With the gate off (the v1.9 default)
-- the agent main loop never calls `post_strategy_signal` and the
-- table stays empty — the reader naturally returns `Ok(vec![])` in
-- that case (V11c). Operators opt in by flipping the TOML to `true`.
--
-- `intended_price_str` is forward-compat per Q9 — v1 strategies emit
-- market-priced signals (`intended_price = NULL`); v2 limit-order
-- shapes will populate it. Stored as TEXT for the same Decimal-as-
-- TEXT contract used by `journal_entries.debit_amount` /
-- `credit_amount` (no float-arithmetic risk; round-trip via Decimal
-- string parsing).
--
-- Anchor risk: zero by construction. The 11 backtest body-SHA-256
-- anchors are computed over committed report bodies; this migration
-- adds a new table with no rows on existing DBs and is never read by
-- the backtest binary.

CREATE TABLE IF NOT EXISTS strategy_signals (
    id                 TEXT PRIMARY KEY,
    ts                 TEXT NOT NULL,       -- RFC3339 with 6-digit microsecond precision
    strategy_id        TEXT NOT NULL,
    venue              TEXT NOT NULL,
    symbol             TEXT NOT NULL,
    side               TEXT NOT NULL,       -- 'buy' | 'sell' | future extensions
    intended_qty_str   TEXT NOT NULL,       -- Decimal as TEXT (Decimal-only money rule)
    intended_price_str TEXT,                -- Decimal as TEXT, NULL for market signals (Q9 forward-compat)
    was_clamped        INTEGER NOT NULL DEFAULT 0,
    clamp_reason       TEXT
);

CREATE INDEX IF NOT EXISTS strategy_signals_ts_idx
    ON strategy_signals(ts);
CREATE INDEX IF NOT EXISTS strategy_signals_vs_idx
    ON strategy_signals(venue, symbol, ts);
CREATE INDEX IF NOT EXISTS strategy_signals_sid_idx
    ON strategy_signals(strategy_id, ts);
