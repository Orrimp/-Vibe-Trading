---
slug: cargo-deps-audit-2026-05-22
date: 2026-05-22
authors: developer (P1.6)
status: complete
related:
  - docs/dev-notes/repo-cleanup-plan-2026-05-22.md
---

# Cargo.toml dep audit — 2026-05-22 (P1.6)

## TL;DR

**CLEAN — 2 minor items noted, no inline fixes required.**

All 6 dep additions are correctly placed and used. Two observations are
logged (redundant dev-dep entries, `rusqlite` / `sqlx` not in
`[workspace.dependencies]`) but neither is a defect — they are low-priority
hygiene items for a future pass.

---

## Per-dep table

| Dep | Crate | Usage site | Feature gate | Runtime or dev | Verdict |
|-----|-------|-----------|--------------|---------------|---------|
| `cost` | `strategy` | `src/llm_forecaster/anthropic_impl.rs:50` — `use cost::{AgentRole, LlmTier}` (non-test) | None (always on) | Runtime (`[dependencies]`) | CLEAN |
| `rusqlite 0.32` | `strategy` | `src/bin/llm_verdict.rs:107,137,156` — `rusqlite::Connection::open`, `rusqlite::params!`, `rusqlite::Result` | None (used only in the `llm_verdict` bin) | Runtime (`[dependencies]`) | CLEAN — see note 1 |
| `pollster` | `strategy` | `src/llm_forecaster/strategy.rs:217,291` — `pollster::block_on(...)` (non-test, on_bar sync path) | None (always on) | Runtime (`[dependencies]`) | CLEAN — see note 2 |
| `uuid` | `strategy` | `src/llm_forecaster/types.rs:304,326,453,546,575,960,1015,1037` — `uuid::Uuid` fields and `::new_v4()` calls in production structs | None (always on) | Runtime (`[dependencies]`) | CLEAN — see note 2 |
| `tokio { rt }` | `strategy` | `src/llm_forecaster/strategy.rs:129` — `tokio::runtime::Handle` field; `src/llm_forecaster/anthropic_impl.rs:186` — `tokio::spawn` | None (always on) | Runtime (`[dependencies]`) | CLEAN |
| `wiremock` | `strategy` | `tests/llm_forecaster_wiremock*.rs`, `tests/llm_forecaster_cost_event.rs`, `tests/llm_forecaster_budget_gate.rs`, `tests/llm_forecaster_cost_cap_short_circuit.rs` — `use wiremock::...` (test-only) | N/A | Dev (`[dev-dependencies]`) | CLEAN |
| `sqlx { sqlite, runtime-tokio }` | `strategy` | `tests/llm_forecaster_wiremock_wave_e.rs:163,174,194,279` — `sqlx::query_as(...)` (test-only) | N/A | Dev (`[dev-dependencies]`) | CLEAN |

---

## Audit / crate sweep: `audit` and `ui`

**`crates/audit/Cargo.toml`** — Wave E added migration 012 (`crates/audit/migrations/012_llm_forecast.sql`). No new external deps were added to `audit/Cargo.toml` as a result; the existing `sqlx { sqlite, runtime-tokio, migrate }` already covers the migration runner. Status: CLEAN.

**`crates/ui/Cargo.toml`** — Wave F (cockpit-training-control) added no new external deps. The `libc = "0.2"` for T-D-N14 PID liveness and `iced_test = "=0.14.0"` / `image-compare = "=0.4"` / `image = "=0.25.6"` for the test harness are pinned, dev-only, and were added in prior sessions. Status: CLEAN (no Wave F additions).

---

## Observations (non-blocking)

### Note 1 — `rusqlite` and `sqlx` are not in `[workspace.dependencies]`

**Observation:** `rusqlite = { version = "0.32", features = ["bundled"] }` appears in both `crates/strategy/Cargo.toml` (runtime `[dependencies]`) and `crates/forecast/Cargo.toml` (dev `[dev-dependencies]`). Both pin `"0.32"` with `features = ["bundled"]`, so version drift is not a current risk. Similarly, `sqlx = { version = "0.8", features = [...] }` appears in 6 crates with inline version pins; all use `"0.8"` consistently.

**Why they are not in workspace:** `rusqlite` (sync, bundled SQLite) and `sqlx` (async) serve different use cases. Crates that need migrations add `"migrate"`, some add `"macros"`. A single workspace entry would need to be a superset of features, which may increase binary size for crates that do not need all features. The current inline pinning is a deliberate tradeoff — acceptable, and consistent across crates.

**Recommendation for future pass:** If the project grows to 3+ crates with `rusqlite` runtime deps, consider promoting to `[workspace.dependencies]` as `rusqlite = { version = "0.32", features = ["bundled"] }` and using `rusqlite.workspace = true` in each crate. For `sqlx`, a workspace entry with the minimal feature set (`["sqlite", "runtime-tokio"]`) could be the base, with additive feature lists in each crate via `{ workspace = true, features = ["migrate"] }`. Not urgent today — 0 drift, 0 breakage.

### Note 2 — `uuid`, `pollster`, and `cost` duplicated in `[dev-dependencies]`

**Observation:** `strategy/Cargo.toml` lists `uuid.workspace = true`, `pollster.workspace = true`, and `cost = { path = "../cost" }` in BOTH `[dependencies]` and `[dev-dependencies]`. Cargo deduplicates these — the dev-dep entries are silently no-ops when the runtime dep already covers the same version. They are harmless but add visual noise.

**Why they are listed twice:** The test files import `uuid`, `pollster`, and `cost` directly. When the Wave A/B/E developer added the dev-dep entries, the runtime entries already existed (or were added in the same wave). Neither entry is wrong; Cargo picks the union.

**Recommendation for future pass:** Remove `uuid`, `pollster`, and `cost` from `[dev-dependencies]` in `crates/strategy/Cargo.toml` — the runtime `[dependencies]` entries already satisfy test builds. This is a cosmetic cleanup (3 lines removed). Deferred: LOW risk, zero functional impact.

---

## Inline fixes applied

None. The build gates all pass clean before and after this audit:

```
cargo build --workspace --features candle  → Finished (0 errors)
cargo clippy --workspace --features candle -- -D warnings  → Finished (0 warnings)
bash scripts/verify_anchors.sh  → ANCHORS PASS (34 / 34)
```

---

## Recommendation for next pass

1. (LOW) Remove the 3 duplicate dev-dep entries from `crates/strategy/Cargo.toml` (`uuid`, `pollster`, `cost`) — cosmetic, ~5 min.
2. (LOW) If `rusqlite` lands in a third crate, promote to `[workspace.dependencies]`. Not yet warranted.
3. (LOW) `sqlx` workspace promotion is deferred until the feature-set divergence across crates stabilises.
