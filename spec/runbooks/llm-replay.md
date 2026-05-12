# LLM Replay Runbook

**Version:** v2.0.0
**Owner:** operator / on-call
**Related code:** `crates/llm/src/replay.rs`, `crates/llm/src/recording.rs`, `crates/llm/src/factory.rs`, `crates/llm/migrations/`

---

## Overview

The v2 LLM stack ships a SQLite-backed record/replay layer so research
and regression workloads can reproduce LLM I/O deterministically. The
cache stores `(request_hash → canonical_response)` pairs. Strict
replay-only at v2.0.0 (D2 operator-locked decision): a cache miss is a
hard `LlmError::ReplayMiss { hash, provider, model }` — no fall-through
to live calls.

The cache layout:

- **Production runtime cache** — `data/llm-replay.db` (configurable via
  `[llm] replay_cache_path`). Written by paper mode's
  `RecordingProvider`; read by research mode's `ReplayProvider`.
- **Committed fixture cache** — `crates/llm/fixtures/replay-v1.db`,
  9 rows (3 providers × 3 roles). Lives in git so `cargo test` is
  hermetic and the smoke binary's research mode is reproducible from
  a fresh checkout.

The schema is migrated through `crates/llm/migrations/` (sqlx). The
`schema_version` column on the `replay_entries` table guards forward
compatibility — see "Schema migration" below.

---

## How research mode uses replay (strict-replay-only at v2.0.0)

When the agent boots with `[mode] = "research"`:

1. `LlmProviderFactory::build(..., Mode::Research, ...)` constructs a
   `ReplayProvider::open(cfg.replay_cache_path)`. No leaf provider is
   built; no API key is required.
2. On every `complete()` call, the provider computes
   `request_hash = sha256(canonical_json(model, system, messages,
   tools, max_tokens, temperature))` and looks it up in the cache.
3. **Hit** — returns the cached `ChatResponse` (byte-identical, all
   token counts and stop reasons preserved).
4. **Miss** — returns `Err(LlmError::ReplayMiss { hash, provider,
   model })`. The error surfaces to the consumer-side error router.

Strict-replay-only matches the determinism contract every v2.0.0
consumer needs (backtests, integration tests, audits). Best-effort
fall-through to live calls is a v3 follow-up brief.

---

## How to refresh the cache

```bash
# 1. Make sure the .local overlay has real API keys.
cp config/agent.toml.local.example config/agent.toml.local
$EDITOR config/agent.toml.local

# 2. Run the smoke binary in paper mode — records every successful
#    complete() into data/llm-replay.db.
cargo run --bin llm-smoke -- --mode paper

# 3. Inspect the new rows.
sqlite3 data/llm-replay.db 'SELECT provider, role, recorded_at FROM replay_entries ORDER BY recorded_at DESC LIMIT 10;'

# 4. (Optional) Copy the freshly recorded cache over the committed
#    fixture so research-mode replays the new shape. Architect
#    approval required for any fixture rotation.
cp data/llm-replay.db crates/llm/fixtures/replay-v1.db
```

The recorder is **append-only** — re-running paper mode never overwrites
an existing `(request_hash)` row.

---

## How to interpret a `LlmError::ReplayMiss(hash)` failure

Cause: the consumer issued a `ChatRequest` whose canonical hash does
not match any row in the cache. Common reasons:

1. **The system prompt drifted.** Even one whitespace change in the
   project / role / dynamic system block flips the hash. To compare:
   ```bash
   # Compute the hash the caller would see.
   cargo test -p llm --test request_hash_test -- --nocapture
   ```
   Then `sqlite3 data/llm-replay.db 'SELECT request_hash FROM replay_entries
   WHERE provider = "anthropic" AND role = "trader";'` and diff.
2. **A new prompt was added** without a fresh paper-mode recording —
   refresh per the procedure above.
3. **The model id rotated.** `(model, system, messages, tools,
   max_tokens, temperature)` are all in the canonical hash. Pinning
   `model = "claude-opus-4-7"` in the test fixture and the production
   request keeps the hash stable across boots.

Canonical SHA-256 of the `(trader, anthropic, claude-opus-4-7)` smoke
request as recorded against the committed fixture at v2.0.0 ship time:

```
sha256(canonical-json):  see `cargo run --bin llm-smoke -- --mode research`
                         output line `request_hash = ...`
```

---

## How to reset the cache

```bash
# Paper-mode reset (deletes the production cache + WAL/SHM sidecars).
cargo run --bin llm-smoke -- --mode paper --reset
```

`--reset` is a no-op under `--mode live` and `--mode research` (the
research cache is committed; resetting it is an architect-gated rotation).

---

## Schema migration (the `schema_version` column)

The `replay_entries` table carries a `schema_version: INTEGER NOT NULL`
column. The v2.0.0 ship value is `1`. On open:

- `ReplayProvider::open` reads the max `schema_version` in the table.
- If `max(schema_version) > SUPPORTED_SCHEMA_VERSION`, the provider
  returns `LlmError::Provider { message: "replay-cache schema vN
  exceeds supported v{SUPPORTED_SCHEMA_VERSION}; upgrade the llm
  crate or downgrade the cache" }`.

Forward-compat is exercised by `crates/llm/tests/replay_schema_forward_compat.rs`
(T1939). When v3 introduces a new column, the new code reads both
v1 and v2 rows; an old binary against a v2 cache surfaces the structured
error rather than silently corrupting state.

To inspect:

```bash
sqlite3 data/llm-replay.db 'SELECT DISTINCT schema_version FROM replay_entries;'
# Expected: 1
```

---

## Related runbooks

- [llm-cost.md](llm-cost.md) — cost-monitoring playbook.
- [kill-switch.md](kill-switch.md) — agent hard-stop procedures.
