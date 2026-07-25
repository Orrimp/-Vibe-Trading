---
adr: 0007
title: v1+ — WASM-plugin hot-load deferred; native dyn-libs and embedded scripting rejected
status: accepted
date: 2026-04-19
supersedes: none
superseded-by: none
---

# ADR-0007: v1+ — WASM-plugin hot-load deferred; native dyn-libs and embedded scripting rejected

## Context

For strategies that need genuinely custom logic — bespoke DL inference,
non-trivial state machines, off-the-shelf research code from Python via
Pyodide — config-driven composition ([ADR-0006](0006-v05-config-driven-composition.md))
isn't enough. v1+ needs a hot-load path that supports arbitrary code.

The two obvious alternatives — native dynamic libraries (`.so`/`.dylib`
via Rust's unstable ABI) and embedded scripting (Rhai, Lua, Rune) —
both have specific failure modes that disqualify them for this project.

## Decision

For v1+ custom-logic strategies, compile to WebAssembly and load via
`wasmtime`. The decision is **deferred** — no WASM strategies ship at
v0.5; the option is reserved for v1+ when a strategy with genuinely
custom logic justifies the deploy complexity.

Properties:
- **Sandboxed.** A buggy strategy cannot crash the agent or leak memory.
- **Language-agnostic.** Rust first; AssemblyScript and eventually
  Python via Pyodide as the strategy author needs.
- **Tradeoff.** Slight per-tick perf overhead vs compiled-in
  (acceptable for non-HFT timeframes); separate deploy step per
  strategy (mitigated by reusing the v0.5 file-watcher pattern).

## Alternatives considered (explicitly NOT chosen)

- **Native `.so` / `.dylib` via Rust dynamic libs.** The Rust ABI is
  unstable across compiler versions. ABI-mismatch crashes happen at
  load time or, worse, in production after weeks of "working".
  Rejected.
- **Embedded scripting** (Rhai, Lua, Rune). Loses Rust's type-safety
  story; adds a parallel error-handling vocabulary that strategy
  authors must learn separately from Rust patterns. Rejected.

## Consequences

- The `Strategy` trait shape (ADR-0005) is the WASM ABI. Adding a
  method post-v0 breaks every shipped plugin. This is why ADR-0005
  is firm about not evolving the trait.
- `wasmtime` becomes a foundation dependency at v1+ — adds compile
  time and binary size. Cost accepted.
- Until v1+ ships an actual WASM strategy, the hot-load path is
  config-driven composition only. Updating that boundary requires
  superseding this ADR.

## Changelog
- 2026-04-19 (architect): initial accept. Extracted from
  `spec/architecture.md` § Strategy registry & hot-loading § v1+ —
  WASM plugins and § Explicitly NOT chosen during Phase 1A Session 4
  (2026-05-13).
