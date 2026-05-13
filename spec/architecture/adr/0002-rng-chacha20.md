---
adr: 0002
title: All RNG must be seeded with ChaCha20 from a config-supplied seed
status: accepted
date: 2026-04-17
supersedes: none
superseded-by: none
---

# ADR-0002: All RNG must be seeded with ChaCha20 from a config-supplied seed

## Context

The 9-anchor body-SHA-256 regression gate requires backtest output to be
byte-identical across runs at the same seed. Any non-deterministic source
in a hot path — wall-clock time, OS RNG, thread-local RNG, hashmap
iteration order — breaks the gate.

The v0 developer round shipped a `thread_rng()` seed for a position-sizing
jitter, which produced different reports on otherwise-equivalent runs and
falsely indicated regression. The fix was mechanical but the rule needed
to be explicit and project-wide before more strategies landed.

## Decision

All randomness in any code reachable from a backtest replay path uses
`rand_chacha::ChaCha20Rng::from_seed(...)` with a seed supplied via
feature config. No `thread_rng()`, no `OsRng`, no `SystemTime`-derived
seed, anywhere in `crates/strategy`, `crates/backtest`, `crates/exec`,
`crates/risk`, or any shared dependency.

Each feature's `feature.md` carries the seed in its frontmatter. The
backtest scenario name + seed pair fully determines the report body.

## Alternatives considered

- **`StdRng::seed_from_u64(...)`.** Stdlib-only, but the underlying PRNG
  isn't formally specified — a future stdlib change could silently change
  the byte output without changing the seed. Rejected.
- **`SmallRng`.** Faster but explicitly documented as non-reproducible
  across versions. Rejected.
- **Hash-derived seed from feature name.** Convenient but hides the
  knob; debugging an anchor diff means asking "what was the seed?",
  which must be a stable answer. Rejected.

## Consequences

- Mechanical enforcement: the developer's determinism checklist
  (`.claude/agents/developer.md`) lists "all RNGs are ChaCha20Rng" as a
  pre-handoff gate. The tester's `verify-anchors` skill catches any
  regression at the body-SHA level.
- Property tests in `crates/core` cover the "same seed → same byte output"
  invariant for the strategy harness.
- Any new strategy must declare its seed in `feature.md` frontmatter
  before its first anchored backtest. Tester refuses to lock an anchor
  for a scenario with an undeclared seed.
- Violations to watch for: `_ = rand::random::<f64>()` (uses thread RNG),
  `HashSet`/`HashMap` iteration into a hashed report body (use
  `BTreeSet`/`BTreeMap` or sort before serializing).

## Changelog
- 2026-04-17 (architect): initial accept. Promoted to a cross-cutting
  invariant in `spec/architecture.md` and `CLAUDE.md`. Extracted to ADR
  during Phase 1A split (2026-05-13).
