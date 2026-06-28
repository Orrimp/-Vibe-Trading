-- Migration 011 — trail-correlation chain for ui-rethink-phase-d-trail v0.1.0
-- (Q1 = (b) ship; R1.1-R1.5; ADR-0031 § Phase D amendment).
--
-- Pure additive — 4 ALTER TABLE ADD COLUMN (all NULL-default) + 1
-- CREATE TABLE IF NOT EXISTS + 4 CREATE INDEX IF NOT EXISTS. No
-- ALTER on any pre-existing column, no UPDATE on any pre-existing
-- row, no backfill. The 22 backtest body-SHA-256 anchors are
-- byte-identical post-mig by construction — none of the anchored
-- reports read the new columns or the new table.
--
-- The companion writers live at:
--   - journal.rs::post_fill_with_signal      (R1.1 + R1.2; extends post_fill)
--   - journal.rs::post_strategy_signal       (R1.3; 6-arg → 7-arg, fwd-compat callers pass None)
--   - journal.rs::post_forecast_event        (R1.4; NEW writer, sibling of post_strategy_signal)
-- And the readers at:
--   - audit::query::trail_for_fill_id        (R6.3; new — 4-way correlated lookup)
--
-- See spec/v1/ui-rethink-phase-d-trail/decomp.md §2 for the column-by-
-- column rationale.

-- R1.1 — journal_transactions.fill_id (the source-of-truth Fill.id)
ALTER TABLE journal_transactions ADD COLUMN fill_id TEXT;
CREATE INDEX IF NOT EXISTS journal_transactions_fill_id_idx
    ON journal_transactions(fill_id);

-- R1.2 — journal_transactions.signal_id (upstream Signal lineage)
ALTER TABLE journal_transactions ADD COLUMN signal_id TEXT;
CREATE INDEX IF NOT EXISTS journal_transactions_signal_id_idx
    ON journal_transactions(signal_id);

-- R1.3 — strategy_signals.forecast_correlation_id (upstream Forecast lineage)
ALTER TABLE strategy_signals ADD COLUMN forecast_correlation_id TEXT;
CREATE INDEX IF NOT EXISTS strategy_signals_forecast_id_idx
    ON strategy_signals(forecast_correlation_id);

-- R1.4 — forecast_events table (the durable side of AuditEvent::ForecastEmitted)
CREATE TABLE IF NOT EXISTS forecast_events (
    correlation_id   TEXT PRIMARY KEY,        -- ForecastOverlay.correlation_id (UUID)
    ts               TEXT NOT NULL,           -- RFC3339 6-digit microsecond (ADR-0004)
    strategy_id      TEXT NOT NULL,           -- StrategyId.0 string
    symbol           TEXT NOT NULL,           -- "BTCUSDT" style
    direction        TEXT NOT NULL,           -- 'up' | 'down' | 'flat'
    confidence       TEXT NOT NULL,           -- Decimal as TEXT (ADR-0003)
    model_revision   TEXT NOT NULL,           -- SHA per ADR-0029
    cache_hit        INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS forecast_events_ts_idx
    ON forecast_events(ts);
CREATE INDEX IF NOT EXISTS forecast_events_strategy_id_idx
    ON forecast_events(strategy_id, ts);
