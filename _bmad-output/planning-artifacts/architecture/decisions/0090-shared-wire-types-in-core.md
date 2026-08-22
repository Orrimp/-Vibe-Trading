---
adr: 0090
title: Activity + halt wire types live in `trading_core`; the producer machinery stays in `agent`
status: accepted
date: 2026-08-22
supersedes: none
superseded-by: none
---

# ADR-0090: Shared wire types in `trading_core` (bug-log #92)

## Context

`crates/ui` declares `agent` only under its `live` feature, but three modules imported it
**unconditionally**:

| file | imported |
|---|---|
| `src/state.rs` | `ActivityEvent`, `HaltReason` |
| `src/lab/activity.rs` | `ActivityEvent`, `ActivityId`, `ActivityKind`, `ActivityOutcome`, `ActivityPhase` |
| `src/widgets/activity_tape.rs` | `ActivityKind`, `ActivityOutcome` (+ the same set in tests) |

So `cargo check -p ui --no-default-features --features fixtures` — a configuration
`crates/ui/Cargo.toml` documents as supported, *"for the gallery-only bin"* — failed with three
`E0432` errors. Nothing in CI, scripts, skills, README or the runbooks built it, so it rotted
unnoticed from roughly the 2026-05-25 promotion of `live` to a default feature until 2026-08-22.
That is bug-log **#92**.

## Decision

**D1 — Move the wire types, not the machinery.** `ActivityId`, `ActivityKind`, `ActivityOutcome`,
`ActivityPhase`, `ActivityEvent` → `trading_core::activity`; `HaltReason` → `trading_core::halt`.
These are plain data: no tokio, no `Instant`, no I/O.

**D2 — `ActivitySender`, `ActivityHandle` and the broadcast channel STAY in `agent`.** They need
tokio, and **`trading_core` has no async dependency** — a property worth preserving deliberately
rather than eroding for convenience. `KillSwitch` itself likewise stays; only the reason enum moves.

**D3 — `agent` re-exports both**, so every existing `agent::ActivityEvent` / `agent::HaltReason`
path keeps compiling. The relocation is source-compatible for all current callers; `ui` was
repointed to `trading_core` because that is the whole point.

**D4 — A CI leg builds the minimal configuration.** `cargo check -p ui --no-default-features
--features fixtures` on the ubuntu leg. Cheap by design — one `cargo check`, no tests: the point is
coverage of a *build configuration*, not of behaviour. Without it, deleting the claim or fixing the
gating would only have changed which side of declared-vs-executed happened to be true.

## Consequences

- The documented minimal build works: `exit 0`, previously `101`.
- `trading_core` gains two small modules and keeps its async-free property.
- It unblocked bug-log **#91**, whose fix lives in exactly that build and could not be
  *compile-verified* before — applying it earlier would have shipped an edit no compiler had checked.
- **A test-target trap, recorded because it cost a regression.** The relocation left
  `NEXT_ACTIVITY_ID` referenced by an `agent` test while the static moved. `cargo check -p agent`
  stayed **green** — the lib compiled; the *test* target did not. Caught only when the full suite
  ran. The test now lives beside the counter it reads, and verification for cross-crate moves must
  use `--all-targets`, which is the flag whose absence hid it.

## References

- Bug-log **#92** (the broken build), **#91** (unblocked by it).
- `crates/core/src/activity.rs`, `crates/core/src/halt.rs` — both carry the reasoning inline.
- CI: `.github/workflows/ci.yml` § "Check ui minimal build".
