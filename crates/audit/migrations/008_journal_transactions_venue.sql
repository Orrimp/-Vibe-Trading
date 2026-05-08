-- Migration 008 — add venue column to journal_transactions (Phase 3 R13).
--
-- Additive: NEW column NULL-defaulted, then a one-statement UPDATE
-- backfills every existing row to 'binance' (every shipped fill on disk
-- today is Binance per Phase 2 venue-handling note). The two-statement
-- shape — additive ADD COLUMN + UPDATE — keeps existing row contents
-- byte-identical otherwise; the `ADD COLUMN ... DEFAULT NULL` clause
-- does not rewrite existing journal-entry rows, only adds storage for
-- the new column. The `UPDATE` then sets the literal 'binance' string
-- in one statement (lowercase to match `Venue::Binance.to_string()` —
-- the trading-core `Display` impl produces snake_case).
--
-- Post-migration, the writer at crates/audit/src/journal.rs::post_fill
-- takes a `venue: Venue` parameter and binds `venue.to_string()` on
-- insert; new fill rows always carry an explicit (snake_case) venue.
-- Other (memo / cost / registry / kill_switch) writers continue to
-- write rows with NULL venue — the column is nullable for backwards
-- compat with the existing memo writers; only fills are venue-attributed
-- in v1.5b plumbing-only state.
--
-- Anchor risk: zero by construction. The 11 backtest body-SHA-256
-- anchors are computed over committed report bodies, not over the
-- audit-DB row layout — the migration cannot shift any anchored byte.

ALTER TABLE journal_transactions ADD COLUMN venue TEXT DEFAULT NULL;
UPDATE journal_transactions SET venue = 'binance' WHERE venue IS NULL;
CREATE INDEX IF NOT EXISTS journal_transactions_venue_idx ON journal_transactions(venue);
