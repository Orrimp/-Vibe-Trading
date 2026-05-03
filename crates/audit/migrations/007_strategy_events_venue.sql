-- T1402 / R8 / Q11 — multi-venue strategy_events.venue column
-- (spec/features/v1-5b-multi-venue.md → Design → Q11).
--
-- Adds a NULLABLE `venue TEXT` column to `strategy_events` so feed-level
-- events (`FeedReconnect`, optionally `KillSwitchTripped`) carry a typed
-- venue attribution. Pre-migration rows have `venue = NULL` — the
-- writer always populates it for new feed-level events; the read path
-- handles `Option<Venue>` semantics.
--
-- Architect's Q11 picks option (a) (schema migration) over option (b)
-- (`<venue>:<symbol>` encoded in `error_summary`) because v1.5b is the
-- load-bearing introduction of the `Venue` type to the system; encoding
-- it in a TEXT column would defeat the type-system change at the audit
-- boundary (the one place structured attribution matters most).
--
-- Purely additive (NULLABLE, no default, no data migration). Idempotent
-- against the sqlx migrator: re-running is a no-op (sqlx tracks
-- migration version). Anchor risk: zero by construction (Q12) — the
-- column is not rendered in any committed report body.

ALTER TABLE strategy_events ADD COLUMN venue TEXT;

CREATE INDEX IF NOT EXISTS strategy_events_venue_idx ON strategy_events(venue);
