-- T1101 — Per-symbol position accounts (per-symbol-position-accounts R1).
--
-- Adds one `assets:position:<SYMBOL>` row to the `accounts` table per
-- pair-symbol in the agent's universe (config/agent.toml [funding].universe).
-- Pre-migration the chart of accounts had a single `assets:position:BTC`
-- row; post-T1102 every fill targets the per-pair account-id, and that row
-- must exist before the first fill writes to it (FK from
-- journal_entries.account_id → accounts.id, see 001_chart_of_accounts.sql:23).
--
-- Purely additive (R3): legacy rows on `assets:position:BTC` stay; readers
-- handle legacy via description-parse (Q4). No money moves (R6).
-- Anchor regression: 11/11 byte-identical (R5, Q7) — body cells do not
-- render account-ids; verified by grep.

INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:BTCUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:ETHUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:BNBUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:SOLUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:XRPUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:ADAUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:DOGEUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:AVAXUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:DOTUSDT', 'asset', 'USDT');
INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('assets:position:LINKUSDT', 'asset', 'USDT');
