---
adr: 0041
title: Trader crate split — reflection-memory consumer moves out of strategy
status: accepted
date: 2026-05-26
supersedes: none
superseded-by: none
extends: 0019, 0039
---

# ADR-0041: Trader crate split — reflection-memory consumer moves out of strategy

## Context

The v3-llm-forecaster ship (REQ-V3-LLM-FORECASTER-001, state =
`shipped-partial` as of 2026-05-22) landed reflection-retrieval directly
inside `crates/strategy/src/llm_forecaster/` across Waves B / C / G
(commits `8c40ab0` + `97b7c39` + `8dcd72c`). This violates the
**R8.1 / R10.8 layering invariant** asserted in
[`spec/v3-llm-forecaster/feature.md`](../../../../spec/v1/v3-llm-forecaster/feature.md)
§ R8.1 and § R10.8 — the analyst-layer strategy crate is forbidden from
consuming reflection-retrieval (memory-aware decision synthesis is a
trader-layer concern per
[`spec/product.md`](../../../../spec/product.md) § Trading-time agent roster line 135-139).

The defensive substring-grep gate at
[`crates/reflection/tests/no_strategy_caller.rs`](../../../../crates/reflection/tests/no_strategy_caller.rs)
(`t1809_no_strategy_crate_consumes_reflection_retrieval`) has been RED
on `main` for the entire current session, naming
`reflection-memory-trader-wiring` as the formal recovery brief. The
gate cites three offending substrings across two files:

- `crates/strategy/src/llm_forecaster/strategy.rs` — `reflection::ReflectionStore`
- `crates/strategy/src/llm_forecaster/types.rs` — `use reflection::… ReflectionStore, retrieve_top_k, …`
- `crates/strategy/src/llm_forecaster/types.rs` — `ForecastContext::from_runtime` body calls `retrieve_top_k`

A fourth touch-point at `crates/strategy/src/registry.rs:130-145`
(the `"llm_forecaster_v3"` TOML-loader match arm) imports
`reflection::NullReflectionStore` — not currently on the gate's
forbidden list, but architecturally part of the same problem
(strategy-crate code that links against `reflection`). The strategy
crate's `Cargo.toml` carries a `reflection = { path = "../reflection" }`
path-dep that exists only to satisfy these consumers.

The recovery is not a substring-rewrite — the layering rule means the
consumer code itself does not belong in the strategy crate. It must
move to a sibling crate that **is** the trader layer (the runtime
decision-synthesis layer downstream of the analyst layer per product.md
§ 3). Three orthogonal decisions need locking here so the developer
wave plan is unambiguous:

1. **Crate boundary** — new sibling vs. extend an existing crate.
2. **Move scope** — clean-cut the entire `llm_forecaster/` subtree vs.
   surgical extraction of only the reflection-touching code paths.
3. **API shape post-move** — does the strategy crate now call into
   trader, or does trader implement `strategy::Strategy` and stay a
   leaf consumer?

## Decision

### D1. New `crates/trader/` workspace crate (Q1=(a))

Create a new sibling crate at `crates/trader/` with the initial public
surface:

```rust
// crates/trader/src/lib.rs
pub mod llm_forecaster; // moved from crates/strategy/src/llm_forecaster/
// (reserved for v0.1.1 — see § Consequences)
// pub mod decision;
// pub mod memory_provider;
```

Workspace `Cargo.toml` adds `"crates/trader"` to `[workspace.members]`
(alphabetical position). The trader crate's `Cargo.toml` carries the
dependencies the move set needs:

- `trading-core` (Bar, Signal, Symbol, Timestamp — already used by
  `llm_forecaster`)
- `strategy` (the `Strategy` trait — `LlmForecasterStrategy` implements
  it; this is the inverse-API per D3 below)
- `reflection` (the legitimate consumer crate post-move)
- `llm` (LlmProvider trait + Budgeted/Recording/Replay decorators)
- `audit` (JournalEntry emission for `kind = "llm_forecast"`)
- dev-deps mirroring the existing strategy crate's llm_forecaster
  test set (rust_decimal, tokio, async-trait, wiremock, etc.)

**`crates/strategy/Cargo.toml` drops** `reflection = { path = "../reflection" }`
and any reflection-related dev-deps. The R8.1 substring-grep gate
becomes **structurally enforceable**: the strategy crate cannot link
against reflection at all, so the forbidden substrings cannot appear
even by accident.

### D2. Clean-cut move of the entire `llm_forecaster/` subtree (Q2=(a))

All 9 source files at `crates/strategy/src/llm_forecaster/` (the
analyst brief miscounted at 8 — actual files enumerated below) move
to `crates/trader/src/llm_forecaster/` via `git mv` to preserve
blame:

```
crates/strategy/src/llm_forecaster/
├── mod.rs               (78 LoC)
├── trait_def.rs         (74 LoC)
├── types.rs             (1105 LoC — includes the offending ReflectionStore + retrieve_top_k consumers)
├── canonicalize.rs      (98 LoC)
├── strategy.rs          (511 LoC — includes LlmForecasterStrategy + ReflectionStore Arc field)
├── anthropic_impl.rs    (672 LoC)
├── prompt.rs            (441 LoC)
├── tool_schema.rs       (255 LoC)
└── verdict.rs           (866 LoC — ADR-0039 L0-L4 classifier)
                          ─────────────
                          4100 LoC total
```

Integration test suites at `crates/strategy/tests/llm_forecaster_*.rs`
(8 files) plus `llm_verdict_priority_tree.rs` and
`llm_forecaster_neutrality.rs` move to `crates/trader/tests/` (10
suites total — the analyst brief said "13 suites" counting non-strategy
crates that are not moving; the move-set is 10). The 9 `[[test]]`
entries in `crates/strategy/Cargo.toml` move to `crates/trader/Cargo.toml`.

The strategy-crate binary `crates/strategy/src/bin/llm_verdict.rs`
(which imports `strategy::llm_forecaster::verdict::*`) moves to
`crates/trader/src/bin/llm_verdict.rs` — it is the L0-L4 verdict
bin authored under ADR-0039 and belongs with the verdict module.

### D3. Inverse API — trader depends on strategy, not the other way round (Q3=(a))

No new trait surface introduced at v0.1.0. The seam between the two
crates is the existing `strategy::Strategy` trait, which knows nothing
about reflection. Post-move:

- `crates/strategy/src/traits.rs` — unchanged. `Strategy` trait stays
  in strategy as the leaf abstraction.
- `crates/trader/src/llm_forecaster/strategy.rs` — implements
  `strategy::Strategy` for `LlmForecasterStrategy`. The `impl Strategy
  for LlmForecasterStrategy` block is byte-identical to the current
  `crates/strategy/src/llm_forecaster/strategy.rs:222-384`.
- The workspace dep DAG becomes `trader → strategy → trading-core`
  (with `trader → reflection`, `trader → llm`, `trader → audit` as
  siblings). No cycle.

The `MemoryProvider` trait sketched in feature.md § R3.3 (option (c))
is deferred to v0.1.1 — premature abstraction at v0.1.0 with no second
memory-consuming strategy in flight. Open a `reflection-memory-provider-trait`
follow-up brief once the second consumer (researcher debate,
risk-team aggregator, etc.) lands.

### D4. Registry-arm decision (T-AR-4 / R2.3)

The `"llm_forecaster_v3"` arm at
[`crates/strategy/src/registry.rs:130-146`](../../../../crates/strategy/src/registry.rs)
**moves out of the strategy crate entirely**. Rationale: the arm
imports `crate::llm_forecaster::{LlmForecasterConfig, LlmForecasterStrategy,
StubForecaster}` and `reflection::NullReflectionStore`; once
`llm_forecaster/` lives in trader, the arm has no business in strategy's
TOML loader.

The TOML-loader contract becomes:

- `crates/strategy/src/registry.rs::load_from_toml` keeps the
  `sma_crossover` arm and the trailing `tracing::warn!("unknown
  strategy kind — skipping")` fallback. The `llm_forecaster_v3` arm
  is **removed** (not replaced with a warn-and-skip; the trader
  registration path is the new contract).
- `crates/trader/src/registry_arm.rs` (new file) exposes a free
  function `pub fn register_llm_forecaster_v3(registry:
  &StrategyRegistry, entry: &StrategyTomlEntry)` that the application
  binary (`crates/ui/src/bin/cockpit_live.rs` + `crates/backtest/src/main.rs`)
  calls before calling `registry.load_from_toml(...)`.
- The application binary thus splits TOML registration in two:
  trader-owned strategies first, then strategy-crate-owned (sma /
  composed / momentum / pairs / etc.).

This satisfies Q4=(a) (tighten the gate) **mechanically without
amending the gate-test substring list** — once the registry arm
leaves strategy, `reflection::NullReflectionStore` no longer appears
in any strategy-crate `.rs` file, and the strategy crate's
`Cargo.toml` no longer carries the `reflection` path-dep, so the
import cannot be re-introduced. The gate-test list stays as-is for
v0.1.0 (the 4 existing substrings); a sibling positive-assertion
test (D5 below) enforces the move landed.

### D5. Gate-test contract — negative on strategy + positive on trader (R5)

Post-move, the gate-test surface is two tests, both in the
`crates/reflection/tests/no_strategy_caller.rs` file (extended,
not split — keeps the regression evidence co-located):

1. **`t1809_no_strategy_crate_consumes_reflection_retrieval`** —
   unchanged body. The 4-substring forbidden list stays as-is. Walks
   `crates/strategy/src/`. Returns to PASS once Wave B's `git mv`
   lands and Wave C rewrites the imports.

2. **`t1810_trader_crate_owns_reflection_retrieval`** — NEW. Walks
   `crates/trader/src/` and asserts the substring `reflection::retrieve_top_k`
   appears in **at least one** `.rs` file (positive assertion;
   prevents accidental deletion of the consumer logic during a
   future refactor). The implementation MUST share the
   `WalkDir + read_to_string + contains` shape with t1809 so a future
   maintainer sees the contract as a sibling.

Q4 gate-test tightening (adding `NullReflectionStore` to the
forbidden list) is **subsumed by D4**: once the registry arm leaves
strategy, the symbol is structurally absent. No list edit needed at
v0.1.0. If a future ship re-introduces a strategy-crate
reflection-consumer, that's a brand-new R8.1 violation and gets its
own brief; tightening the list pre-emptively risks blocking a future
legitimate use case (e.g. a strategy-side `NullReflectionStore` test
double in an entirely different module).

### D6. Audit emission stays at the call site (Q5=(a))

The trader crate owns audit emission for LLM forecasts. The
`JournalEntry { kind: "llm_forecast", … }` write-site sits inside
`LlmForecasterStrategy::on_bar` (current location:
`crates/strategy/src/llm_forecaster/strategy.rs:286-320`) and moves
with the rest of the file under D2. This is anchor-additive only —
the audit migration `011_llm_forecast.sql` already shipped under
REQ-V3-LLM-FORECASTER-001 Wave G; no audit-crate changes needed.

### D7. Documentation surface

The following documentation surfaces update during Wave E (additive,
non-anchored):

- `crates/reflection/src/lib.rs` doc-comment lines 11-18 (the "Q4 =
  report-only" block that names this brief) — point at trader.
- `spec/v3-llm-forecaster/feature.md` — append `## Errata` section
  acknowledging the R8.1 / R10.8 violation and naming this brief +
  ADR-0041 as the recovery. (Per CLAUDE.md non-negotiables, anchored
  reports under `spec/v3-llm-forecaster/reports/` are byte-immutable;
  `feature.md` is NOT a report and takes additive edits per
  spec-update normal protocol.)
- `spec/product.md` § Trading-time agent roster — optional footnote
  acknowledging `crates/trader/` is the implementation crate for the
  trader layer. Architect-recommended: yes, one-line footnote.
- `spec/architecture.md` — module-map subsection adds
  `crates/trader/` as the runtime trader-layer crate. Architect-owned.
- `spec/v3-llm-forecaster/decomp.md` — leaves historical evidence
  intact (per K6 mitigation option (iii)). The path references inside
  it are now historical; the trace.toml row carries the forward
  pointer to this ADR.

## Alternatives considered

1. **Q1=(b) Extend `crates/agent/`.** Rejected — `agent` is the
   dev-time multi-agent orchestration crate per `AGENT.md`. Mixing
   the runtime trader-layer into the dev-time orchestration crate
   would conflate two unrelated concerns and complicate future
   binary partitioning (the cockpit binary should not link
   dev-time orchestration to ship LLM forecasts).

2. **Q1=(c) Extend `crates/strategy/` with a `trader/` sub-module.**
   Rejected — the substring-grep gate walks `crates/strategy/src/`
   recursively; a sub-module does not satisfy the gate. Bypassing
   the gate with a `#[cfg(...)]` carve-out invites silent regression
   and defeats the structural enforcement that comes for free with
   a separate crate (D1).

3. **Q1=(d) Extend `crates/forecast/`.** Rejected — `forecast` is
   feature-engineering (GARCH MLE, TCN inference, Parkinson target
   derivation, etc.). Decision synthesis (memory-aware "should I go
   long?") is a different layer entirely; mixing them would
   pollute the forecast crate's purpose and make future feature-bin
   additions harder.

4. **Q2=(b) Surgical extraction of only the reflection-touching code
   paths.** Rejected — would split a cohesive module across crate
   boundaries (the `LlmForecasterStrategy` synthesizes a memory-aware
   decision; the prompt builder reads the retrieved lessons; the
   verdict classifier reads the cache hits; the audit emitter reads
   the cost events). Each of these touches the same `ForecastContext`
   shape. Splitting at the reflection surface alone would leave
   `ForecastContext::from_runtime` (the reflection-consumer) in
   trader and `ForecastContext` itself (the consumer of
   `from_runtime`) in strategy. Hostile to readability + future
   maintenance.

5. **Q2=(c) Feature flags.** Rejected — feature flags don't satisfy
   substring-grep (the source bytes still contain the forbidden
   imports), and they re-introduce conditional compilation surface
   that the gate-test specifically defends against.

6. **Q3=(b) New `trader::MemoryAwareTrader` trait at v0.1.0.**
   Rejected — premature abstraction. No second implementation exists
   yet. The `Strategy` trait alone is the seam, and trader
   implements it. A second consumer in v0.1.1+ will surface the right
   shape for a `MemoryProvider` trait (Q3 option (c)) which gets its
   own brief and ADR at that point.

7. **D4 registry-arm: keep warn-and-skip vs. remove.** Considered;
   chose remove. A warn-and-skip leaves the symbol `llm_forecaster_v3`
   meaningful inside the strategy registry, inviting future
   developers to re-add a stub. Removing the arm makes the contract
   explicit: trader-layer strategies register via trader's API, not
   via strategy's TOML loader.

8. **D5 gate-test split into two files.** Rejected — the regression
   evidence belongs together. Co-locating t1809 + t1810 in
   `no_strategy_caller.rs` keeps the negative + positive invariants
   one `cargo nextest run -p reflection --test no_strategy_caller`
   away. Splitting them would obscure the layering rule.

## Consequences

**New crate:**

- `crates/trader/` — new workspace member. Initial public surface:
  `pub mod llm_forecaster;`. Path-deps: `trading-core`, `strategy`,
  `reflection`, `llm`, `audit`. Cargo.toml hosts the 9 `[[test]]`
  entries moved from strategy + the `[[bin]] name = "llm_verdict"`
  entry.

**Modified files (cargo / workspace):**

- `Cargo.toml` (workspace root) — additive: `crates/trader` member.
- `crates/strategy/Cargo.toml` — subtractive: removes `reflection`
  path-dep + 9 `[[test]]` entries + (if present) the `[[bin]]`
  entry for `llm_verdict`. The `reflection-fake-store` dev-dep (if
  any) also moves to trader.

**Moved files (git mv preserving blame):**

- `crates/strategy/src/llm_forecaster/{mod, trait_def, types, canonicalize, strategy, anthropic_impl, prompt, tool_schema, verdict}.rs`
  → `crates/trader/src/llm_forecaster/*` (9 files, ~4100 LoC).
- `crates/strategy/tests/llm_forecaster_{audit_tick, budget_gate, cost_cap_short_circuit, cost_event, neutrality, payload, signal_mapping, wiremock, wiremock_wave_e}.rs`
  + `llm_verdict_priority_tree.rs` → `crates/trader/tests/*` (10
  suites, ~98 integration tests by trace.toml line 1070).
- `crates/strategy/src/bin/llm_verdict.rs` → `crates/trader/src/bin/llm_verdict.rs`.

**Modified files (post-move imports):**

- `crates/strategy/src/lib.rs` — remove `pub mod llm_forecaster;`
  line + the doc-comment reference to `crates/strategy/src/llm_forecaster/`.
- `crates/strategy/src/registry.rs` — remove the `"llm_forecaster_v3"`
  match arm (lines 100, 111-119, 126-146); remove the doc-comment
  line about it; remove the `use reflection::NullReflectionStore`
  inside the arm.
- `crates/trader/src/registry_arm.rs` — NEW. Hosts the moved arm as
  a free function `register_llm_forecaster_v3`.
- 10 moved integration tests — `s/use strategy::llm_forecaster::/use trader::llm_forecaster::/g`
  and equivalent for fully-qualified paths
  (`strategy::llm_forecaster::DEFAULT_MODEL_ID` → `trader::llm_forecaster::DEFAULT_MODEL_ID`).
- `crates/ui/src/assistant/state.rs:21` — doc-comment-only update
  (replace `crates/strategy::llm_forecaster::types::LlmForecast`
  with `crates/trader::llm_forecaster::types::LlmForecast`).
- Any application binary that calls
  `crates/strategy/src/registry.rs::load_from_toml` for the
  `llm_forecaster_v3` kind — must add a paired call to
  `trader::register_llm_forecaster_v3(...)`. Wave D inventory step
  (T-D-N7 below) enumerates these.

**New tests:**

- `crates/reflection/tests/no_strategy_caller.rs::t1810_trader_crate_owns_reflection_retrieval`
  — sibling positive-assertion test (R5.3 / D5 above). Same
  WalkDir + read_to_string + contains shape; asserts `reflection::retrieve_top_k`
  appears in at least one `crates/trader/src/**/*.rs` file.

**Cross-phase implications:**

- The trader crate becomes the canonical home for future memory-aware
  decision-synthesis modules. v0.1.1's `MemoryProvider` trait (the
  deferred Q3 option (c)) lands here when the second consumer ships.
- Future v0.2.0 work (researcher debate, risk team, portfolio
  manager per product.md § Trading-time agent roster) all gain a
  natural home in `crates/trader/` (or sibling new crates that
  depend on it).
- The 34 body-SHA-256 anchors stay byte-identical — pure
  package-level refactor; no scenario body bytes touch. The
  per-tester step `bash scripts/verify_anchors.sh` is the gate.

**Enforced by:**

- `cargo nextest run -p reflection --test no_strategy_caller` —
  both t1809 (negative, RED → GREEN) and t1810 (positive, NEW)
  PASS.
- `cargo metadata --format-version 1 | jq '.workspace_members'` +
  edge check that `strategy` has no `reflection` path-dep.
- `cargo build --workspace --bins` — confirms no binary import
  regression (K4 mitigation).
- `bash scripts/verify_anchors.sh` — `ANCHORS PASS (34 / 34)` at
  M-FINAL (H2 falsifier).
- `cargo nextest run -p trader` — 98 LLM-forecaster integration
  tests PASS post-move (H3 falsifier).

**What breaks if this is violated:**

- A future PR re-introduces a `use reflection::*` in
  `crates/strategy/src/` → t1809 RED, CI blocks. The recovery path
  is documented in this ADR: route to trader, not bypass the gate.
- A future PR accidentally deletes the `retrieve_top_k` call from
  trader during a refactor → t1810 RED, CI blocks. The recovery
  path is documented: trader IS the memory-aware decision layer;
  if memory retrieval is genuinely no longer needed, write a
  superseding ADR removing both t1810 and the consumer.
- A future PR re-adds a `reflection` path-dep to
  `crates/strategy/Cargo.toml` → t1809 may still PASS depending on
  source usage, but the cargo-metadata edge check (above) catches
  it. Architect-recommended: extend `scripts/spec_lint.py` with a
  cargo-metadata edge invariant in a v0.1.1 hardening pass.

**What this enables:**

- The gate-test red on `main` flips green; the workspace re-enters a
  ship-able state for unrelated work.
- The trader-layer crate seam unblocks future v0.2.0 work
  (researcher debate, risk team, portfolio manager) that needs the
  same layering boundary.
- The strategy crate's `Cargo.toml` shrinks (one fewer path-dep);
  the substring-grep gate becomes structurally enforceable (you
  literally cannot consume reflection from strategy because strategy
  doesn't depend on it).

## References

- [ADR-0019](0019-v2-llm-strategy.md) — v2 LLM-strategy foundation;
  `LlmForecasterStrategy` builds on the `crates/llm` v2.0.0 surface.
  This ADR moves the v3 LLM-forecaster implementation to trader but
  the LLM-provider abstraction stays in `crates/llm`.
- [ADR-0039](0039-llm-forecaster-verdict-criteria.md) — L0-L4
  verdict criteria; the `verdict.rs` classifier moves with the rest
  of `llm_forecaster/` under D2. No verdict shape change.
- [`spec/reflection-memory-trader-wiring/feature.md`](../../../../spec/v1/reflection-memory-trader-wiring/feature.md)
  — analyst brief; R1-R7 + K1-K8 + H1-H5 + Q1-Q7.
- [`spec/reflection-memory-trader-wiring/tasks.md`](../../../../spec/v1/reflection-memory-trader-wiring/tasks.md)
  — architect M-T1 decomposition into Waves A-D + M-FINAL +
  M-PRESENTER.
- [`spec/v3-llm-forecaster/feature.md`](../../../../spec/v1/v3-llm-forecaster/feature.md)
  § R8.1 + § R10.8 — the violated invariants this ADR recovers.
- [`crates/reflection/tests/no_strategy_caller.rs`](../../../../crates/reflection/tests/no_strategy_caller.rs)
  — gate-test source; t1810 added in the same file under D5.
- [`spec/product.md`](../../../../spec/product.md) § Trading-time agent roster
  line 135-139 — Trader-agent layer definition.

## Changelog

- 2026-05-26 (architect): initial accept. Locks D1 new
  `crates/trader/` crate (Q1=(a)), D2 clean-cut move of all 9 source
  files + 10 test suites + 1 bin (Q2=(a) — corrects analyst's "8
  files / 13 suites" miscount to actual 9 / 10), D3 inverse-API via
  existing `strategy::Strategy` trait (Q3=(a); `MemoryProvider`
  trait deferred to v0.1.1), D4 registry-arm full removal + move
  to `trader::register_llm_forecaster_v3` (resolves T-AR-4 +
  subsumes Q4 mechanically), D5 t1810 positive-assertion sibling
  test (R5.3), D6 trader-owns-audit (Q5=(a)), D7 documentation
  surface (Q7=(a) errata append). Cross-refs
  `REQ-REFLECTION-TRADER-001` in `spec/trace.toml`.
