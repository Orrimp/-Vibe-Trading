-- T1919 — Replay cache schema (Design § Q8b).
--
-- v1 of the deterministic-replay store. The cache is a content-addressed
-- KV table keyed by `request_hash` (SHA-256 hex over canonical JSON of
-- `(model, system, messages, tools, max_tokens, temperature)` per
-- Design § Q8a — `correlation_id` is excluded). One row per unique
-- request body × provider × model, so re-recording the same logical
-- request idempotently overwrites (`INSERT OR REPLACE`).
--
-- The `schema_version` column is the forward-compat hook (Q8b):
-- `ReplayProvider::open` asserts `schema_version <= SUPPORTED_SCHEMA_VERSION`
-- (the module-level constant at `crates/llm/src/replay.rs`); a v3 column
-- add bumps the constant to 2 and extends this migration with a sibling
-- `002_*.sql` rather than mutating this file.
--
-- The `created_at` / `updated_at` columns use 6-digit fractional ISO
-- timestamps for the same reason the audit ledger does (Q4 / discipline
-- rule 4 in AGENT.md): SQLite ORDER BY ties on second-precision
-- timestamps are a real bug we paid for.

CREATE TABLE IF NOT EXISTS llm_replay (
    request_hash      TEXT PRIMARY KEY NOT NULL,    -- 64-char SHA-256 hex
    schema_version    INTEGER NOT NULL,             -- 1 at v2.0.0
    provider          TEXT NOT NULL,                -- e.g. "anthropic", "openai", "ollama"
    model             TEXT NOT NULL,                -- e.g. "claude-opus-4-7"
    request_json      TEXT NOT NULL,                -- canonical JSON (debugging surface)
    response_json     TEXT NOT NULL,                -- ChatResponse serialised via serde_json
    recorded_at       TEXT NOT NULL                 -- 6-digit fractional ISO (RFC-3339)
);

CREATE INDEX IF NOT EXISTS llm_replay_provider_idx ON llm_replay(provider);
