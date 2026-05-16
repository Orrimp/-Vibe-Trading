-- T-M1-4 — Generic replay-cache schema (v2.5).
--
-- Shared between crates/llm (ChatResponse) and crates/forecast (ForecastResponse).
-- The cache is content-addressed: one row per unique canonical-JSON hash of
-- the request parameters. Re-recording the same request idempotently
-- overwrites via INSERT OR REPLACE.
--
-- schema_version is the forward-compat hook: consumers assert
-- schema_version <= SUPPORTED_SCHEMA_VERSION on open.
--
-- Timestamps use 6-digit fractional ISO-8601 (RFC-3339) to avoid SQLite
-- ORDER BY ties on second-precision values (ADR-0004 rule).

CREATE TABLE IF NOT EXISTS replay_cache (
    request_hash      TEXT PRIMARY KEY NOT NULL,   -- 64-char SHA-256 hex
    schema_version    INTEGER NOT NULL,            -- 1 at v2.5
    namespace         TEXT NOT NULL,               -- e.g. "llm", "kronos" — discriminator
    request_json      TEXT NOT NULL,               -- canonical JSON of request params (debug surface)
    response_json     TEXT NOT NULL,               -- serde_json of V (the response type)
    recorded_at       TEXT NOT NULL               -- 6-digit fractional ISO (RFC-3339)
);

CREATE INDEX IF NOT EXISTS replay_cache_namespace_idx ON replay_cache(namespace);
