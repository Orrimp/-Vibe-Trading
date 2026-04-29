-- Funding-rate observations (T613 — v1 Q2).
--
-- Stores hourly snapshots from Binance /fapi/v1/premiumIndex.
-- Observation-only: strategy does NOT read from this table.
-- The MomentumStrategy ignores funding rates per Q2 resolution.

CREATE TABLE IF NOT EXISTS funding_rates (
    id               TEXT PRIMARY KEY,   -- uuid v4
    symbol           TEXT NOT NULL,      -- e.g. "BTCUSDT"
    funding_rate     TEXT NOT NULL,      -- Decimal serialised as string (no f64)
    funding_ts       TEXT NOT NULL,      -- RFC3339 — venue timestamp of last funding
    next_funding_ts  TEXT NOT NULL,      -- RFC3339 — next scheduled funding time
    poll_ts          TEXT NOT NULL       -- RFC3339 — local time of the REST poll
);

CREATE INDEX IF NOT EXISTS funding_rates_symbol_idx ON funding_rates(symbol, funding_ts);
CREATE INDEX IF NOT EXISTS funding_rates_poll_ts_idx ON funding_rates(poll_ts);
