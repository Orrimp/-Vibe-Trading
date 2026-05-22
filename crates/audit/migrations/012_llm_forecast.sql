-- Migration 012 — llm_forecast journal entries for v3-llm-forecaster Wave E
-- (T-D-N(E1); R7.1.2 — JournalEntry { kind: "llm_forecast", payload }).
--
-- Pure additive — 1 CREATE TABLE IF NOT EXISTS + 2 CREATE INDEX IF NOT EXISTS.
-- No ALTER on any pre-existing column, no UPDATE on any pre-existing row.
-- The 34 backtest body-SHA-256 anchors are byte-identical post-mig by
-- construction — none of the anchored reports read this table.
--
-- The companion writer lives at:
--   audit::journal::post_llm_forecast  (R7.1.2 — new)
-- The AuditTick event is:
--   AuditEvent::LlmForecastEmitted     (R7.1.3 — new variant)
--
-- Row shape:
--   id               TEXT PRIMARY KEY — UUID v4 string
--   ts               TEXT NOT NULL    — RFC3339 6-digit microsecond (ADR-0004)
--   strategy_id      TEXT NOT NULL    — "llm_forecaster_v3"
--   symbol           TEXT NOT NULL    — e.g. "BTCUSDT"
--   correlation_id   TEXT NOT NULL    — echoed from ForecastContext (UUID)
--   rating           TEXT NOT NULL    — "STRONG_BUY"|"BUY"|"HOLD"|"SELL"|"STRONG_SELL"
--   confidence       TEXT NOT NULL    — Decimal as TEXT (ADR-0003)
--   horizon          TEXT NOT NULL    — "one_hour" (only value at v0.1.0)
--   reasoning_trace  TEXT NOT NULL    — full trace text (50–2000 chars)
--   trace_sha256     TEXT NOT NULL    — lowercase 64-hex SHA-256 of reasoning_trace
--   cited_lesson_ids TEXT NOT NULL    — JSON array of card_id strings
--   tokens_in        INTEGER NOT NULL — input tokens billed
--   tokens_out       INTEGER NOT NULL — output tokens billed
--   tokens_cached_in INTEGER NOT NULL — cache-read tokens (Anthropic)
--   cost_usd         TEXT NOT NULL    — Decimal as TEXT; actual cost for this call
--   forecaster_name  TEXT NOT NULL    — "llm_forecaster_impl"
--   model_id         TEXT NOT NULL    — e.g. "claude-haiku-4-5-20251001"
--
-- See spec/v3-llm-forecaster/decomp.md § T-AR-1 step 8 + R7.1 for
-- the full payload contract.

CREATE TABLE IF NOT EXISTS llm_forecast_entries (
    id               TEXT PRIMARY KEY,
    ts               TEXT NOT NULL,
    strategy_id      TEXT NOT NULL,
    symbol           TEXT NOT NULL,
    correlation_id   TEXT NOT NULL UNIQUE,
    rating           TEXT NOT NULL,
    confidence       TEXT NOT NULL,
    horizon          TEXT NOT NULL,
    reasoning_trace  TEXT NOT NULL,
    trace_sha256     TEXT NOT NULL,
    cited_lesson_ids TEXT NOT NULL DEFAULT '[]',
    tokens_in        INTEGER NOT NULL DEFAULT 0,
    tokens_out       INTEGER NOT NULL DEFAULT 0,
    tokens_cached_in INTEGER NOT NULL DEFAULT 0,
    cost_usd         TEXT NOT NULL DEFAULT '0',
    forecaster_name  TEXT NOT NULL,
    model_id         TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS llm_forecast_entries_ts_idx
    ON llm_forecast_entries(ts);

CREATE INDEX IF NOT EXISTS llm_forecast_entries_symbol_strategy_idx
    ON llm_forecast_entries(symbol, strategy_id, ts);
