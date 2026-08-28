---
adr: 0091
title: Pin the Rust toolchain — an unpinned compiler is an uncontrolled build input
status: accepted
date: 2026-08-23
supersedes: none
superseded-by: none
---

# ADR-0091: Pin the Rust toolchain

## Context

CI installed `dtolnay/rust-toolchain@stable`: whatever stable was published that
morning. Nothing in the repository said which compiler builds it. The canonical
box runs **1.94.1** (2026-03-25); upstream stable had reached **1.98.0**
(2026-08-18) — a four-release gap that nobody had chosen.

It surfaced as a red CI leg. `crates/core/tests/compile_fail/*.stderr` are
`trybuild` expectations: byte-exact rustc diagnostics. `compile_fail_tests`
failed on the macOS and Ubuntu legs while passing on every developer machine.

**Measured, not inferred** — the CI failure was reproduced locally:

```bash
cargo +1.98.0 test -p trading_core --test trybuild   # FAILED (1 of 4)
cargo test -p trading_core --test trybuild           # ok (1.94.1)
```

1.94 reports 8 private fields on the `Order` struct literal; 1.98 reports 7 — it
excludes the field that IS supplied — and re-formats the spans. Same source, same
test, different compiler.

Finding it took the annotation tooling added the same day, because run logs need
repo admin: before that, this failure was the word "failure" beside a step name.

## Decision

**Pin the toolchain in `rust-toolchain.toml` at `1.94.1`**, with `rustfmt` and
`clippy` components. `rustup` honours it for every invocation — CI, the canonical
box, and every contributor — so all three build with the same compiler.

CI's `dtolnay/rust-toolchain@stable` step is left in place: with the file present
it resolves to the pinned channel, and the step still provides the components.

**Rejected — re-bless the `.stderr` files under 1.98 and keep floating.** It
fixes exactly one stable release. The failure returns on 1.99, on someone else's
schedule, and each round tempts a maintainer to "just re-bless" a gate whose
whole value is that it is exact. It also leaves the deeper problem untouched: the
evidence corpus would still be produced by a compiler nobody selected.

## Consequences

- **The compiler joins the controlled inputs.** This repository pins 119 anchored
  report bodies by SHA and asserts byte-identical reproduction. Leaving the
  compiler floating while hashing its output is the same declared-versus-actual
  shape this codebase keeps finding: the thing claimed to be reproducible had an
  uncontrolled input.
- **Raising the pin becomes deliberate.** A bump is one commit that changes the
  channel, re-runs `verify_anchors.sh`, and re-blesses the trybuild expectations
  together. That is the correct cost for changing a build input.
- **A HYPOTHESIS for bug-log #93, explicitly not a claim.** #93 records that the
  code stopped reproducing the frozen evidence, with a cause that bisect placed
  before this session's fixes and never identified. A compiler upgrade on the
  canonical box is now a *candidate* explanation and is cheap to test: check out
  the commit that produced the pins, build under the pinned toolchain, and
  re-run. If the hashes return, the drift was never in the code. **Nobody should
  record this as the cause until that runs** — #93's own moral is that a
  plausible story is not a measurement.
- Contributors on a different stable get an automatic download on first build.

## References

- Bug-log: **#93** (code-vs-evidence drift, cause open — see the hypothesis above).
- `crates/core/tests/compile_fail/` — the trybuild expectations this protects.
- CI annotation tooling: `scripts/ci_run_annotated.sh` — how the failure became
  visible without repo admin.
