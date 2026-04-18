---
date: 2026-04-17
author: developer
---

# Developer Week 1 Notes — v0 Paper SMA

## Naming Collision: `core` crate shadows stdlib `core`

The workspace crate named `core` shadows the Rust standard library's `core`
module in several contexts:

1. **`thiserror` proc-macro in dependent crates** — generates `::core::fmt::Display` 
   which resolves to our crate instead of stdlib. Fixed by using
   `trading_core = { package = "core", path = "../core" }` in all dependent
   crates' Cargo.toml files.

2. **Doctest context in `core` crate itself** — `::core::` inside rustdoc
   examples resolves to our crate. Fixed by setting `doctest = false` in
   `[lib]` in `core/Cargo.toml`.

3. **Integration tests (`tests/*.rs`) for `core`** — `proptest!` macros use
   `::core::` internally. Fixed by moving proptest suites into `src/tests/`
   (inside-crate `#[cfg(test)]` modules) instead of `tests/` (integration
   tests).

4. **UI crate** — ui-designer was using `use core::` which shadowed stdlib.
   Fixed by renaming all `use core::` to `use trading_core::` in ui source
   files. **Coordination note for ui-designer: all `core` imports must use
   `trading_core::` (the alias set in `ui/Cargo.toml`).**

### Recommendation for architect review
Consider renaming the crate from `core` to `trading_core` in the workspace
to avoid these systemic issues. The package can stay named `core` with the
`trading_core` alias, but source files all use `trading_core::` consistently.
This would be a search-and-replace across source files only, not a structural
change. Filed for v0.5 discussion.

---

## Blocker: `sqlx-ledger` is Postgres-only (T05)

`sqlx-ledger` v0.11.14 (latest) uses Postgres exclusively. The `cargo.toml`
shows `postgres` in features; the README and source confirm there is no SQLite
backend. The architecture decision (confirmed 2026-04-17) specified
`sqlx-ledger` with SQLite as the T05 backing store.

### Investigation
- `cargo info sqlx-ledger` shows no SQLite feature flag.
- Source (`src/lib.rs`) shows Postgres connection string examples only.
- The `cala-ledger` sibling (also Postgres-only) was already ruled out.

### Workaround implemented (T05)
Implemented a hand-rolled double-entry ledger using raw `sqlx` + SQLite.
The public API surface (`audit::query`, `audit::journal::post_fill`,
`audit::bootstrap::chart_of_accounts`) is identical to what the architect
specified. The SQLite backend is hidden behind `audit::backend::SqliteBackend`
so a future swap to `sqlx-ledger` (if it gains SQLite) or `cala-ledger`
(if Postgres becomes acceptable) is a one-file change.

Schema is stored as embedded migrations via `sqlx::migrate!`.

### Dependency change
Removed `sqlx-ledger` (Postgres-only) from workspace dependencies.
Added `sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }`.

**Escalation to architect:** The `sqlx-ledger` choice needs revisiting.
Three options:
1. Keep hand-rolled SQLite ledger (current workaround) — fits single-binary
   deploy goal perfectly.
2. Wait for `sqlx-ledger` to add SQLite support (unknown timeline).
3. Switch to `cala-ledger` + Postgres — breaks single-binary deploy goal.

Proceeding with option 1 pending architect feedback.

---

## UI Crate `trading_core` rename required

**For ui-designer:** The `core` crate is accessed via `trading_core` alias in
all dependent crates (including `ui`). All imports that were `use core::...`
must be `use trading_core::...`. This has been mechanically applied to the
existing UI source files found as of 2026-04-17. Any new UI files created
after this date must use `use trading_core::...` consistently.

The alias is set in `crates/ui/Cargo.toml`:
```toml
trading_core = { package = "core", path = "../core" }
```

---

## Dependency version resolutions

| Issue | Resolution |
|-------|------------|
| `rand_chacha = "0.4"` not found | Updated to `"0.9"` (actual latest stable) |
| `sqlx-ledger` Postgres-only | Replaced with raw `sqlx` + SQLite |

---

## Status of T01–T12 as of this writing

| Task | Status | Notes |
|------|--------|-------|
| T01 | done | Workspace + all 13 crates; `cargo check` + `cargo deny` pass |
| T02 | done | All core types; clippy clean; serde round-trips pass |
| T03 | done | trybuild compile-fail: `Money<Usdt> + Money<Btc>` rejects |
| T04 | done | proptest suite: `Order::new` all invariants |
| T05 | in-progress | SQLite via raw sqlx (sqlx-ledger blocker documented) |
| T06 | pending | |
| T07 | pending | |
| T08 | pending | |
| T09 | pending | |
| T10 | pending | |
| T11 | pending | |
| T12 | pending | |
