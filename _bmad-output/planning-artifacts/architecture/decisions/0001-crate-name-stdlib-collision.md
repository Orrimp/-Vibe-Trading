---
adr: 0001
title: Crate package names must not shadow Rust stdlib crates
status: accepted
date: 2026-04-17
supersedes: none
superseded-by: none
---

# ADR-0001: Crate package names must not shadow Rust stdlib crates

## Context

The workspace's foundation crate was originally named `core`. Rust 2024's
prelude imports resolve bare `core::` paths to the stdlib `::core::`
crate, but when a workspace member is *itself* named `core` the two names
collide inside any compilation unit that pulls it in directly — most
visibly in `rustdoc` doc-test harnesses and in macro-expanded code that
emits `::core::fmt` / `::core::write` (e.g. `thiserror::Error`).

A per-crate `doctest = false` escape hatch silences `cargo test -p <crate>`
but does NOT protect `cargo test --workspace --doc`, which bypasses the
flag and fails with `E0433` errors inside macro expansions.

The v0 developer hit this twice. The friction is silent: the build is
green with `cargo test`, then breaks under `--doc` only after macros
expand.

## Decision

No workspace member may take a package name that shadows a Rust stdlib
crate. Forbidden names: `core`, `alloc`, `std`, `test`, `proc_macro`.

The foundation crate's package name is `trading_core`. Imports across the
workspace read `use trading_core::{Symbol, Order, …};`.

**Directory name vs. package name.** The crate directory stays
`crates/core/` — directory names don't affect imports and don't collide
with anything, and renaming forces a touchy path update across every
`Cargo.toml`. The single source-of-truth is the `[package] name` field.
If a future change wants 1:1 directory↔package mapping, that's a
separate PR with its own ADR.

## Alternatives considered

- **Keep `package = "core"` with `trading_core = { package = "core" }`
  aliases in every consumer.** Works for `--all-targets` but breaks
  `--doc`. Fails open: any new crate that forgets the alias compiles
  against the workspace `core` and breaks confusingly when a macro
  emits a `::core::` path. Rejected.
- **Keep `package = "core"` + workspace-wide `doctest = false`.** Masks
  the failing gate instead of fixing it; alias trap persists. Rejected.
- **Rename `crates/core/` → `crates/trading_core/` in the same change.**
  Cleaner long-term but the extra churn (git history, Cargo.lock, every
  `path = "../core"`) is not worth it against the single-knob
  `[package] name` rename. Deferred.

## Consequences

- Mechanical enforcement: `scripts/precheck.sh` greps every workspace
  member's `[package] name` and exits non-zero on the forbidden set.
  Run by `rust-build` and by the architect's library-compatibility
  checklist before locking any new dependency.
- Documentation discipline: `architect.md` § Library/crate compatibility
  checklist includes the "no stdlib-name shadowing" line as a hard
  reject criterion.
- If a future crate genuinely needs the name `core` (e.g. for external
  consumers expecting `core::Symbol`), file a superseding ADR — do not
  rename quietly.

## Changelog
- 2026-04-17 (architect): initial accept. Extracted from
  `spec/architecture.md` § Naming conventions during Phase 1A split
  (2026-05-13).
