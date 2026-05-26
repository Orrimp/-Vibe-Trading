---
slug: reflection-memory-trader-wiring
status: draft
owner: analyst
updated: 2026-05-25
version: 0.1.0
predecessor: v3-llm-forecaster v0.1.0 (shipped-partial)
parent: REQ-V3-LLM-FORECASTER-001 (R8.1 / R10.8 layering rule)
gate_test: crates/reflection/tests/no_strategy_caller.rs::t1809_no_strategy_crate_consumes_reflection_retrieval
---

# Reflection-memory trader wiring — recover the R8.1 layering invariant

> **Hygiene-gate recovery brief.** v3-llm-forecaster Waves B / C / G
> (commits 8c40ab0, 97b7c39, 8dcd72c) wired reflection-retrieval directly
> into `crates/strategy/src/llm_forecaster/`, violating R8.1 (analyst-
> layer strategies MUST NOT consume reflection retrieval). The gate-test
> `t1809_no_strategy_crate_consumes_reflection_retrieval` names this
> brief as the formal recovery path. **No second-guessing the
> architectural decision** — the v3 authors already foresaw this
> triage. Scope: move reflection-retrieval into a new `trader` layer,
> invert the API so `strategy` calls into `trader` (not the other way
> round), and return the gate-test to PASS at M-FINAL.

## Why

### The R8.1 layering rule

Per [`spec/v3-llm-forecaster/feature.md` § R8.1](../v3-llm-forecaster/feature.md)
and [§ R10.8](../v3-llm-forecaster/feature.md):

> "R10.8 — No reflection-memory writer touch — C5 is a *read* consumer
> of `top_k`. ... The `reflection-memory-trader-wiring` follow-up
> brief (deferred per
> [reflection-memory Q4 line 13-18](../../crates/reflection/src/lib.rs))
> is **superseded by this feature** — C5 IS the trader-wiring."

The v3 brief asserted C5 *is* the trader-wiring, but the implementation
landed it inside `crates/strategy/src/llm_forecaster/` — which is the
**analyst-layer strategy crate** per
[`spec/product.md` § Trading-time agent roster](../product.md#trading-time-agent-roster).
Per product.md § 3 line 135-139, the *Trader agent* is a separate runtime
layer downstream of the analyst layer:

```
analysts (parallel) → researcher debate → trader → risk team → portfolio manager → exec
                                          ^^^^^^^
                                          THIS layer
```

Reflection retrieval is **trader-layer concern** (memory-aware decision
synthesis); the analyst layer (where `LlmForecasterStrategy` currently
sits) should produce opinions from data only. The gate-test enforces
the substring-level invariant.

### Current red gate

```
crates/reflection/tests/no_strategy_caller.rs::t1809_no_strategy_crate_consumes_reflection_retrieval

Q4 / R8.1 violation — strategy crate consumes reflection retrieval:
[
    "crates/strategy/src/llm_forecaster/strategy.rs contains forbidden substring `reflection::store::`",
    "crates/strategy/src/llm_forecaster/strategy.rs contains forbidden substring `reflection::ReflectionStore`",
    "crates/strategy/src/llm_forecaster/types.rs contains forbidden substring `reflection::retrieve_top_k`",
]
```

Forbidden substrings (per gate-test source at
[`crates/reflection/tests/no_strategy_caller.rs:28-33`](../../crates/reflection/tests/no_strategy_caller.rs)):

1. `reflection::retrieve_top_k`
2. `reflection::store::`
3. `reflection::ReflectionStore`
4. `reflection::store::sqlite`

Audit of current `crates/strategy/src/` (2026-05-25):

| File | Line | Forbidden substring(s) hit | Move target |
|------|------|----------------------------|-------------|
| `llm_forecaster/strategy.rs` | 120, 154 | `reflection::ReflectionStore` (Arc field + ctor arg) | trader layer |
| `llm_forecaster/strategy.rs` | 182 | `reflection::store::NullReflectionStore` | trader layer (or test-only) |
| `llm_forecaster/types.rs` | 40 | `use reflection::{… ReflectionStore, retrieve_top_k, …}` | trader layer |
| `llm_forecaster/types.rs` | 461 | `reflection::RetrievalError` (not forbidden — informational) | trader layer (consequential move) |
| `llm_forecaster/types.rs` | 496-516 | `ForecastContext::from_runtime` body calls `retrieve_top_k` | trader layer |
| `registry.rs` | 134, 142 | `use reflection::NullReflectionStore` (NOT forbidden — `Null` variant) | trader layer (cleanup) |

**Note**: only `strategy.rs` + `types.rs` trip the substring grep
today (3 hits). `registry.rs` uses `reflection::NullReflectionStore`
which is *not* on the forbidden list (no `store::` substring on that
line) — but it's still strategy-crate reflection-consumer code and
should move with the rest under the layering principle. Architect
M-T1 confirms whether the gate-test should be tightened to catch it
or whether it's acceptable as a registry-only escape hatch.

### Strategic context

The trader-layer recovery is **not just a hygiene fix** — it sets up
the runtime topology product.md § Trading-time agent roster has
been pointing at since 2026-04-17. Future v0.2.0 work (researcher
debate, risk team, portfolio manager) all depend on the trader-as-
separate-crate boundary. This brief is the seam.

## Requirements

> Numbered + testable. Architect M-T1 ratifies these into a
> `decomp.md` Wave plan; developer waves cite by R-id.

### R1 — Define the trader-layer surface

- **R1.1** — **Q1 default = (a) new `crates/trader/` crate**. Create
  a new workspace crate at `crates/trader/` with the following initial
  surface:
  ```rust
  // crates/trader/src/lib.rs
  pub mod decision;        // DecisionContext + Decision + DecisionError
  pub mod memory_provider; // MemoryProvider trait + impls
  pub mod llm_forecaster;  // moved from crates/strategy/src/llm_forecaster/
  ```
  Crate-level lints inherit from `crates/strategy/` (no `f64`, etc.).
  Q1 alternates: (b) extend `crates/agent/` (rejected — agent is the
  multi-agent dev-time crate per AGENT.md, not the runtime trader);
  (c) extend `crates/strategy/` with a `trader/` sub-module (rejected —
  doesn't satisfy substring-grep gate); (d) extend `crates/forecast/`
  (rejected — forecast is feature-engineering, not decision synthesis).

- **R1.2** — Workspace `Cargo.toml` adds `crates/trader` to
  `[workspace.members]`. The crate dependencies are:
  - `trading-core` (Bar / Signal / SignalKind / Symbol / Timestamp)
  - `strategy` (the `Strategy` trait — trader implements `Strategy`)
  - `reflection` (the offending dep — now legal here)
  - `llm` (LlmProvider trait + budgeted/recording/replay decorators)
  - `audit` (JournalEntry emission — moved from strategy)

- **R1.3** — `crates/strategy/Cargo.toml` **removes** the
  `reflection = { path = "../reflection" }` dep. The R8.1 substring-grep
  gate becomes structurally enforceable: the strategy crate cannot
  link against reflection at all, so the substrings cannot appear.

- **Acceptance:** `cargo build -p trader` clean; `cargo build -p
  strategy` clean with `reflection` removed; `cargo metadata` shows
  `strategy → reflection` edge is GONE.

### R2 — Move set (precise files + symbols)

> Architect M-T1 confirms the exact moves; this is the analyst-strawman.

- **R2.1** — Move `crates/strategy/src/llm_forecaster/` (entire 8-file
  subdirectory) → `crates/trader/src/llm_forecaster/`:
  - `mod.rs`
  - `trait_def.rs` (LlmForecaster async trait)
  - `types.rs` (ForecastContext, LlmForecast, Rating, Confidence,
    Horizon, LlmForecasterError, LessonCardRef, CostEventRef,
    TechnicalIndicators, RecentDecision, StubForecaster,
    LlmForecasterConfig + CACHE_SCHEMA_VERSION / PROMPT_TEMPLATE_VERSION /
    DEFAULT_MODEL_ID / TOP_K_LESSONS / DEFAULT_TIMEOUT_MS /
    DEFAULT_FIRE_EVERY_N_BARS constants)
  - `canonicalize.rs` (request_hash helpers)
  - `strategy.rs` (LlmForecasterStrategy + STRATEGY_ID const)
  - `anthropic_impl.rs` (LlmForecasterImpl)
  - `prompt.rs` (CachedSystemPromptBuilder consumer)
  - `tool_schema.rs` (propose_forecast ToolSchema)
  - `verdict.rs` (ADR-0039 L0-L4 classifier — LVerdict / LlmForecastRow /
    LlmWindowStats / aggregate_rows / classify_l)

- **R2.2** — Re-exports preserved at `crates/trader/src/lib.rs` so
  callers (registry, backtest scenarios, tests) only need a path-rewrite:
  `strategy::llm_forecaster::…` → `trader::llm_forecaster::…`. The
  public API surface is byte-identical post-move.

- **R2.3** — Strategy-crate registry adaptation: `crates/strategy/src/
  registry.rs` lines 130-146 (the `"llm_forecaster_v3"` match arm)
  moves to a new `crates/trader/src/registry_arm.rs` exposed as a
  function `register_llm_forecaster_v3(registry: &StrategyRegistry,
  entry: &StrategyTomlEntry)` that the application binary calls. The
  strategy registry's `load_from_toml` keeps its other arms (sma_crossover,
  etc.); the `"llm_forecaster_v3"` arm becomes an explicit no-match
  warning (`tracing::warn!("llm_forecaster_v3 is now a trader-layer
  strategy; register via trader::register_llm_forecaster_v3 from your
  bin")`). Architect M-T1 decides whether to keep this warning or
  fully remove the arm.

- **R2.4** — Test files that currently live at
  `crates/strategy/tests/llm_forecaster_*.rs` (13 suites per trace.toml
  REQ-V3-LLM-FORECASTER-001 `tests` column line 1070) move to
  `crates/trader/tests/llm_forecaster_*.rs`. The 98 integration tests
  must continue passing post-move (byte-identical assertion contracts).

- **Acceptance:** the offending 3 substrings are absent from every
  `.rs` file under `crates/strategy/src/`; `cargo nextest run -p
  trader` runs the same 98 integration tests with the same PASS count.

### R3 — Inverse API: strategy → trader call shape

> The key insight is that *nobody currently calls into trader from
> strategy*. The `strategy::Strategy` trait is satisfied by
> `LlmForecasterStrategy`; consumers (registry, backtest engine) hold
> a `Box<dyn Strategy>` and call `on_bar(&Bar)`. Once
> `LlmForecasterStrategy` moves to `trader`, the `Strategy` trait stays
> in `strategy` — `trader::LlmForecasterStrategy` implements
> `strategy::Strategy`. This is the **inverse-API**: trader depends on
> strategy (the trait abstraction), not the other way around.

- **R3.1** — `crates/strategy/src/traits.rs` stays unchanged. The
  `Strategy` trait is the seam: it knows nothing about reflection.

- **R3.2** — `crates/trader/src/llm_forecaster/strategy.rs` (after move)
  implements `strategy::Strategy` for `LlmForecasterStrategy`. The
  `impl Strategy for LlmForecasterStrategy` block is byte-identical
  to the current `crates/strategy/src/llm_forecaster/strategy.rs:222-384`.

- **R3.3** — **No new trait surface introduced at v0.1.0**. Q3 default
  = (a). Q3 alternates surveyed for future waves:
  - (b) A new `trader::MemoryAwareTrader` trait that `LlmForecasterStrategy`
    implements alongside `Strategy` — rejected at v0.1.0 (premature
    abstraction; no second implementation exists yet).
  - (c) A new `trader::MemoryProvider` trait that wraps reflection
    retrieval and is injected into `LlmForecasterStrategy` — DEFERRED
    to v0.1.1. This is the right shape once a second memory-consuming
    strategy lands (researcher debate, etc.); analyst-recommends opening
    it as a v0.1.1 brief once the second consumer appears.

- **R3.4** — Application-binary wiring (`crates/backtest/src/main.rs`,
  `crates/agent/src/main.rs`, etc.) imports
  `trader::{LlmForecasterStrategy, LlmForecasterConfig, …}` instead of
  `strategy::llm_forecaster::…`. Architect M-T1 inventories the import
  sites (analyst counts ~6 from the v3 trace.toml `crates` column).

- **Acceptance:** the strategy crate has zero knowledge of the trader
  crate or reflection crate (per `cargo metadata`); the trader crate
  links both reflection and strategy; consumers route through trader.

### R4 — `Cargo.toml` workspace + crate updates

- **R4.1** — Root `Cargo.toml`: add `"crates/trader"` to
  `[workspace.members]` in the alphabetical position.

- **R4.2** — `crates/trader/Cargo.toml` (new file) — analyst-strawman:
  ```toml
  [package]
  name = "trader"
  version = "0.1.0"
  edition = "2024"

  [dependencies]
  trading-core = { path = "../core" }
  strategy = { path = "../strategy" }
  reflection = { path = "../reflection" }
  llm = { path = "../llm" }
  audit = { path = "../audit" }
  # ... mirror dev-deps from the existing crates/strategy/Cargo.toml
  # for llm_forecaster_* (rust_decimal, tokio, async-trait, etc.)
  ```
  Architect M-T1 owns the exact dep set.

- **R4.3** — `crates/strategy/Cargo.toml` — **remove**
  `reflection = { path = "../reflection" }` and any reflection-related
  dev-deps (`reflection-fake-store` etc.).

- **Acceptance:** `cargo build --workspace` clean; `cargo metadata` shows
  `strategy` has no path-dep on `reflection`.

### R5 — Gate-test recovery contract (load-bearing)

- **R5.1** — Post-move, `t1809_no_strategy_crate_consumes_reflection_retrieval`
  returns to PASS — the 3 offending substrings are absent from every
  `.rs` file under `crates/strategy/src/` because the entire
  `llm_forecaster/` subtree has moved.

- **R5.2** — **Architect M-T1 decides** whether to tighten the gate-test
  to also forbid `reflection::NullReflectionStore` (the `registry.rs:134`
  hit that's currently not on the forbidden list). Analyst-strawman:
  yes, tighten it as part of this brief — the registry should not
  reference reflection at all post-R2.3.

- **R5.3** — Add a sibling gate-test
  `t1810_trader_crate_owns_reflection_retrieval` at
  `crates/reflection/tests/no_strategy_caller.rs` (NEW test, same file):
  positive assertion that the `crates/trader/src/` tree DOES contain
  `reflection::retrieve_top_k` (proves the move landed in the right
  place; prevents accidental deletion of the consumer logic).

- **R5.4** — CI gate: existing `scripts/spec_lint.py` or
  `scripts/verify_anchors.sh` invocation should run
  `cargo nextest run -p reflection --test no_strategy_caller`
  in pre-PR. Architect M-T1 confirms the exact wiring.

- **Acceptance:** the gate-test passes at M-FINAL; a sibling positive-
  assertion gate-test (R5.3) also passes.

### R6 — Non-regression contract (additive-zero)

- **R6.1** — **All 34 body-SHA-256 anchors stay byte-identical**. The
  move is a pure refactor — no strategy/backtest scenario output
  changes. The v3-llm-forecaster `top10-202[34]-fy-llm-forecaster-realdata`
  scenarios (planned at v0.2.0+ per REQ-V3-LLM-FORECASTER-001 anchors
  line 1071) are unaffected — they haven't shipped yet.

- **R6.2** — **All 98 LLM-forecaster integration tests stay PASS** (per
  trace.toml REQ-V3-LLM-FORECASTER-001 `tests` column). The move is
  package-level only; test bodies are byte-identical apart from import
  path rewrites.

- **R6.3** — **Phase F UI byte-identity preserved**. The Assistant slot
  promotion (`R9.1-R9.3` from v3-llm-forecaster) operates over types
  re-exported through `trader`; no UI code changes. Phase F default-
  disabled config remains byte-identical.

- **R6.4** — **`Strategy` trait + registry surface byte-identical**.
  Strategy-crate public API unchanged; only internal implementations
  move out. The 11 other strategies (sma_crossover, v0.5 composed,
  v1 momentum, v1.5a pairs, v2.5 TCN overlay, v2.5a PatchTST overlay,
  v3-volatility-forecaster, etc.) are untouched.

- **R6.5** — **No iced bump**. The vendored `iced_tiny_skia` fork
  stays untouched per
  [CLAUDE.md operator-lock 2026-05-20](../../CLAUDE.md#vendored-dependencies).

- **R6.6** — **No audit-ledger writer touch**. Audit migrations
  unchanged; the v3 migration 011 (if shipped) stays in
  `crates/audit/migrations/`.

- **R6.7** — **No reflection-memory writer touch**. The lesson-card
  writer pipeline + `ReflectionStore` trait + 3-method surface
  (`upsert` / `top_k` / `count`) all stay byte-identical.

- **R6.8** — **`spec-lint` contribution = 0**. No new lint violations.

- **Acceptance:** `scripts/verify_anchors.sh` → ANCHORS PASS (34/34) at
  M-FINAL; `cargo nextest run --workspace` shows no new failures.

### R7 — Documentation + spec sync

- **R7.1** — `spec/v3-llm-forecaster/feature.md` gets a `## Errata`
  (or amendment block at the end) noting that R8.1 / R10.8 were
  asserted but not enforced at Wave B-G, and that this brief is the
  recovery. Per ADR-0038 § D6 anchor-additive rule + CLAUDE.md
  non-negotiable on anchored report files, the v3 reports under
  `spec/v3-llm-forecaster/reports/` are NOT touched — only the
  brief itself gets the errata note (it's not an anchored report).

- **R7.2** — `spec/product.md` § Trading-time agent roster (line 105+)
  optionally gets a footnote acknowledging the implementation now
  lives in `crates/trader/`. Analyst-strawman: yes; architect M-T1
  decides scope.

- **R7.3** — `spec/architecture.md` gets a new sub-section under §
  module map naming `crates/trader/` as the runtime trader-layer
  crate. Architect M-T1 owns the prose.

- **R7.4** — `crates/reflection/src/lib.rs` doc-comment line 11-18 (the
  "Q4 = report-only" block that names this brief) gets updated to:
  > "The trader crate's `LlmForecasterStrategy` is the first consumer
  > of `retrieve_top_k`. The `no_strategy_caller.rs` defensive grep
  > continues to enforce that the strategy crate stays consumer-free."
  Developer M-DEV touches this comment.

- **Acceptance:** the four documentation surfaces (v3 errata, product
  footnote, architecture sub-section, lib.rs comment) all updated.

## Q-questions (operator-decide)

### Q1 — New crate vs extend existing

(a) **New `crates/trader/` workspace crate** — clean separation;
    long-term-correct topology per product.md § Trading-time agent
    roster.
(b) Extend `crates/agent/` — rejected (agent = dev-time multi-agent
    orchestration; mixing concerns).
(c) Extend `crates/strategy/` with a `trader/` sub-module — rejected
    (doesn't satisfy substring-grep; would require gate-test bypass).
(d) Extend `crates/forecast/` — rejected (forecast = feature
    engineering, not decision synthesis).

**Analyst-recommended: (a)** — new crate is the clean cut. Cost is
~1 day of Cargo.toml wiring + path rewrites; pays back forever.

### Q2 — Move scope (all of llm_forecaster/, or just the reflection-touching bits?)

(a) **Move the entire `crates/strategy/src/llm_forecaster/` subtree**
    (8 files + tests) to `crates/trader/src/llm_forecaster/` — clean
    cut; the whole LLM-forecaster is trader-layer by product.md
    definition.
(b) Move only `types.rs::ForecastContext::from_runtime` + `strategy.rs::
    SymbolState` reflection-touching code; keep the rest in strategy.
    Rejected — splits a cohesive module across crate boundaries; the
    `LlmForecasterStrategy` synthesizes a memory-aware decision (per
    product.md § 3 Trader agent line 135-139), which IS trader-layer.
(c) Move just the reflection imports behind a feature flag. Rejected —
    feature flags don't satisfy substring-grep.

**Analyst-recommended: (a)** — clean cut. Architect M-T1 confirms whether
sub-modules with different layer concerns exist inside `llm_forecaster/`
that should stay; analyst's read of the 8 files says no.

### Q3 — Inverse-API shape (does v0.1.0 introduce a new trait?)

(a) **No new trait** — `trader::LlmForecasterStrategy` implements
    `strategy::Strategy` directly; the move is a pure refactor.
(b) Introduce `trader::MemoryAwareTrader` trait that
    `LlmForecasterStrategy` implements alongside `Strategy`. Rejected
    at v0.1.0 (premature abstraction; no second impl exists).
(c) Introduce `trader::MemoryProvider` trait that wraps reflection
    retrieval and is injected into trader strategies. **Defer to
    v0.1.1** — analyst-recommended once the second memory-consuming
    strategy lands (researcher debate, etc.).

**Analyst-recommended: (a)** — minimum-blast-radius v0.1.0. The
`MemoryProvider` trait (c) is the right shape for v0.1.1+, but
introducing it before a second consumer is premature.

### Q4 — Gate-test tightening (add `NullReflectionStore` to the forbidden list?)

(a) **Tighten the gate-test** — add `reflection::NullReflectionStore`
    + `reflection::Null` to the forbidden list so the registry.rs:134
    + 142 hits are caught structurally.
(b) Leave the gate-test as-is — registry's `NullReflectionStore` is a
    test-double escape hatch and not a real reflection consumer.

**Analyst-recommended: (a)** — once R2.3 lands and the registry's
`"llm_forecaster_v3"` arm is gone from strategy, there's no reason
for `NullReflectionStore` to appear in strategy at all. Tightening
prevents regression. Architect M-T1 decides whether to bundle the
tightening into this brief's M-DEV or as a follow-up.

### Q5 — `audit` crate dep in trader (does trader emit audit rows directly?)

(a) **Trader owns audit emission for LLM forecasts** — `LlmForecaster::
    forecast()` returns; trader writes the `JournalEntry { kind:
    "llm_forecast", … }` and the `AuditTick` broadcast. Default per
    R1.2 trader Cargo.toml.
(b) Strategy crate owns audit emission; trader returns rich types
    and strategy translates. Rejected — would re-introduce reflection-
    touching code in strategy (via the lesson-card-citation field on
    JournalEntry).

**Analyst-recommended: (a)** — audit is downstream of decision; sits
naturally in trader.

### Q6 — Sequencing relative to other Active briefs

(a) **Ship this before any new v3-llm-forecaster Wave D work**
    (replay-cache + backtest scenarios). Wave D currently DEFERRED
    in v3-llm-forecaster's trace row line 1072 — no urgency conflict.
(b) Ship in parallel with lab-yahoo-realdata v0.1.1 (the queued
    follow-up at backlog Active line 438). No file overlap; parallel
    is safe.
(c) Block on architect bandwidth — sequence after the next
    high-priority active feature ships.

**Analyst-recommended: (a) + (b)** — ship as soon as architect M-T1
has bandwidth; parallel with lab-yahoo-realdata v0.1.1 is safe. The
gate-test red on main is a P0 hygiene problem and should not linger.

### Q7 — Errata location for the v3-llm-forecaster brief

(a) **Append a `## Errata` section at the end of
    `spec/v3-llm-forecaster/feature.md`** noting the R8.1 violation
    landed and this brief is the recovery.
(b) Mutate the v3 R8.1 / R10.8 text in-place. Rejected — that's
    historical evidence; mutating it loses the audit trail.
(c) Leave the v3 brief alone; the trace-row state column carries the
    story.

**Analyst-recommended: (a)** — preserve history, add forward-pointer.
Per CLAUDE.md non-negotiables, anchored reports under
`spec/v3-llm-forecaster/reports/` are byte-immutable; the
`feature.md` is NOT a report and can take additive edits per
spec-update normal protocol.

## K-risk register

### K1 — Move-set scope creep (modules that look like they belong but don't)

**Risk:** The `crates/strategy/src/` tree has files outside
`llm_forecaster/` (sma_crossover.rs, tcn_overlay_momentum.rs, registry.rs,
traits.rs, etc.) that might appear to be trader-layer at second glance.
**Severity:** LOW — the substring-grep gate is precise; only the
3-substring violators move. Architect M-T1's M-T1 audit lists exact
move-set.
**Mitigation:** Q2=(a) clean-cut on `llm_forecaster/` only; everything
else stays. R2.4 enumerates the 13 integration test suites that move.

### K2 — Test-fixture path rewrites

**Risk:** The 98 integration tests rely on import paths like
`use strategy::llm_forecaster::types::Rating;`. Mass rewrite to
`use trader::llm_forecaster::types::Rating;` is mechanical but error-
prone; a missed file fails compilation only when its test runs.
**Severity:** MEDIUM (operational).
**Mitigation:** Developer M-DEV uses `rg -l` + `sed` (with eyeballing)
to enumerate import sites; architect M-T1 confirms the inventory
matches the trace.toml `tests` column. Pre-flight `cargo check
--workspace --tests` catches misses before push.

### K3 — Cargo workspace dependency cycle risk

**Risk:** `trader` depends on `strategy` (for the `Strategy` trait);
`strategy` previously depended on `reflection` (which is now gone).
Is there any path where `strategy` could need a `trader`-defined
type, creating a cycle?
**Severity:** LOW — the `Strategy` trait is a leaf abstraction; the
crate has no consumer-of-trader needs. Architect M-T1 audits with
`cargo metadata`.
**Mitigation:** Architect verifies `cargo metadata --format-version 1
| jq '.workspace_members'` shows a clean DAG.

### K4 — Application binary wiring (cockpit-live / backtest / lab)

**Risk:** Six+ binaries (cockpit-live, backtest, lab, fetch_yahoo_klines,
llm_forecaster_rerecord, etc.) import `strategy::llm_forecaster::…`.
Missing a binary leaves a compile error or worse — a silent runtime
that loads zero LLM strategies.
**Severity:** MEDIUM (operational).
**Mitigation:** R3.4 inventories the binary sites; developer M-DEV
runs `cargo build --workspace --bins` to confirm all bins compile
post-move. The `cockpit-smoke` skill catches runtime regressions.

### K5 — Anchor risk

**Risk:** Strategies that don't import reflection (sma_crossover, v1
momentum, etc.) produce backtest reports that anchor under
`spec/anchors.toml`. If the move accidentally changes a Strategy
trait method signature or a Bar/Signal type re-export, anchors break.
**Severity:** LOW — refactor is package-level; no trait or value-type
changes.
**Mitigation:** R6.1 + R6.4 + tester M-FINAL runs
`scripts/verify_anchors.sh` and gates on `ANCHORS PASS (34 / 34)`.

### K6 — `decomp.md` from v3-llm-forecaster references obsolete paths

**Risk:** `spec/v3-llm-forecaster/decomp.md` has ~720 lines naming
`crates/strategy/src/llm_forecaster/…` paths throughout. Per
CLAUDE.md non-negotiable, anchored reports are byte-immutable —
but is `decomp.md` an anchored report?
**Severity:** LOW — `decomp.md` is architect-owned and NOT under
`spec/v3-llm-forecaster/reports/`. It's architect-spec material;
can be updated via spec-update normal protocol.
**Mitigation:** Architect M-T1 either (i) updates decomp.md path
references in-place, (ii) appends an Errata block, or (iii) leaves
historical evidence and lets the trace.toml carry the story. Architect
M-T1 decides at handoff.

### K7 — Gate-test path expectations

**Risk:** `crates/reflection/tests/no_strategy_caller.rs` hard-codes
`crates/strategy/src/` as the walk root (lines 21-22). If we add the
R5.3 positive-assertion test for `crates/trader/src/`, we must add
similar relative-path logic.
**Severity:** LOW — known surface; one-line addition.
**Mitigation:** R5.3 architect-recommends a single helper `fn
walk_substrings(root: &Path, needles: &[&str]) -> Vec<String>` used
by both the negative (strategy) and positive (trader) tests.

### K8 — Concurrent feature interference (lab-yahoo-realdata v0.1.1)

**Risk:** lab-yahoo-realdata v0.1.1 (backlog Active line 438) touches
`crates/ui/src/lab/runner.rs` + `crates/backtest/`. This brief touches
`crates/strategy/` + new `crates/trader/`. No direct file overlap; but
both modify workspace `Cargo.toml`.
**Severity:** LOW — `[workspace.members]` is order-tolerant.
**Mitigation:** Q6=(b) parallel-ship is safe; rebase on main daily.

## H-hypothesis register

### H1 — gate-test recovery
**Hypothesis:** Post-M-FINAL, the gate-test
`t1809_no_strategy_crate_consumes_reflection_retrieval` returns to
PASS.
**Falsifier:** Run `cargo nextest run -p reflection --test
no_strategy_caller` — exit code 0 + assertion PASS.
**Confidence:** HIGH (the refactor is mechanically deterministic).

### H2 — additive-zero anchor preservation
**Hypothesis:** All 34 existing body-SHA-256 anchors stay byte-identical.
**Falsifier:** `scripts/verify_anchors.sh` → `ANCHORS PASS (34 / 34)`.
**Confidence:** HIGH (no scenario body bytes touched).

### H3 — test-suite preservation
**Hypothesis:** All 98 LLM-forecaster integration tests stay PASS post-move.
**Falsifier:** `cargo nextest run -p trader` shows 98 passed; 0 failed.
**Confidence:** HIGH (import path rewrites only; no logic touch).

### H4 — clean workspace dep DAG
**Hypothesis:** Post-move, `cargo metadata` shows no cyclic dep
between strategy/trader/reflection.
**Falsifier:** `cargo metadata` parses + `cargo build --workspace` clean.
**Confidence:** HIGH (DAG is straightforwardly enforceable).

### H5 — schedule
**Hypothesis:** 3-5 days wall-clock from M-T1 to M-FINAL (no LLM
calls; pure refactor).
**Falsifier:** Tester M-FINAL completes within 5 calendar days of
architect M-T1 handoff.
**Confidence:** MEDIUM (mechanical refactor, but workspace-wide path
rewrites have surprises).

## Non-regression contract

> Per R6 + the v3-llm-forecaster precedent (R10).

- [ ] **34 / 34 anchors byte-identical** (R6.1; tester runs
      `scripts/verify_anchors.sh`).
- [ ] **98 / 98 LLM-forecaster integration tests PASS** (R6.2; tester
      runs `cargo nextest run -p trader`).
- [ ] **22 + Phase F snapshot baselines byte-identical** (R6.3;
      `cargo nextest run -p ui` includes visual_snapshots).
- [ ] **Strategy crate public API byte-identical** (R6.4; SemVer
      check via `cargo-semver-checks` if available).
- [ ] **No iced bump** (R6.5; vendor/iced_tiny_skia/ unchanged).
- [ ] **No audit writer touch** (R6.6; `git diff crates/audit/migrations/`
      empty).
- [ ] **No reflection writer touch** (R6.7; `git diff
      crates/reflection/src/writer/ crates/reflection/src/store/`
      empty).
- [ ] **`spec-lint` contribution = 0** (R6.8).
- [ ] **Gate-test `t1809_no_strategy_crate_consumes_reflection_retrieval`
      returns to PASS** (R5.1).
- [ ] **NEW gate-test `t1810_trader_crate_owns_reflection_retrieval`
      PASS** (R5.3).

## Handoff

**HANDOFF → architect**

**Input files:**
- `spec/reflection-memory-trader-wiring/feature.md` (this file)
- `spec/reflection-memory-trader-wiring/tasks.md` (M0 / M-T1 / M-DEV /
  M-FINAL / M-PRESENTER stubs)
- `spec/v3-llm-forecaster/feature.md` § R8.1 + § R10.8 (the violated
  invariant)
- `crates/reflection/tests/no_strategy_caller.rs` (the gate-test)
- `crates/strategy/src/llm_forecaster/` (the 8-file move set + 13
  test suites)

**Open questions for architect M-T1:**
- Q1 — confirm new `crates/trader/` crate (analyst-recommended (a)).
- Q2 — confirm clean-cut of entire `llm_forecaster/` subtree
  (analyst-recommended (a)).
- Q3 — confirm no new trait at v0.1.0 (analyst-recommended (a);
  v0.1.1 brief for `MemoryProvider`).
- Q4 — confirm gate-test tightening to include `NullReflectionStore`
  (analyst-recommended (a)).
- Q5 — confirm trader owns audit emission (analyst-recommended (a)).
- Q6 — confirm sequencing parallel with lab-yahoo-realdata v0.1.1
  (analyst-recommended).
- Q7 — confirm v3-llm-forecaster errata location (analyst-recommended
  `## Errata` append).
- K6 — decomp.md path-update strategy.
- R5.2 — gate-test tightening scope (bundle vs follow-up).

**Trace row:** `REQ-REFLECTION-TRADER-001` at `proposed` state in
`spec/trace.toml`.

**Cost estimate:** 3-5 days wall-clock (architect M-T1 ~0.5 day,
developer M-DEV ~2 days, tester M-FINAL ~0.5 day, presenter ~0.5 day).
No LLM costs (pure refactor; no model calls).

**Anchor risk:** LOW (additive-zero by construction).

**Gate severity:** P0 (red on main; CI/test failure blocks PRs).

## Implementation

Developer M-DEV completed 2026-05-26. All 12 T-D-N* tasks executed in
Waves A → B → C → D per tasks.md plan.

### Files created

- `crates/trader/Cargo.toml` — new workspace member with path-deps:
  `trading_core`, `strategy`, `reflection`, `llm`, `audit`, `cost`,
  plus dev-deps matching the moved test suites.
- `crates/trader/src/lib.rs` — pub re-exports of all public symbols.
- `crates/trader/src/registry_arm.rs` — `register_llm_forecaster_v3`
  free function (ADR-0041 § D4).

### Files moved (git mv, blame preserved)

**Source files (9):**
`crates/strategy/src/llm_forecaster/{mod,trait_def,types,canonicalize,strategy,anthropic_impl,prompt,tool_schema,verdict}.rs`
→ `crates/trader/src/llm_forecaster/`

**Bin (1):**
`crates/strategy/src/bin/llm_verdict.rs` → `crates/trader/src/bin/llm_verdict.rs`

**Integration tests (10):**
`crates/strategy/tests/llm_forecaster_{audit_tick,budget_gate,cost_cap_short_circuit,cost_event,neutrality,payload,signal_mapping,wiremock,wiremock_wave_e}.rs`
+ `llm_verdict_priority_tree.rs` → `crates/trader/tests/`

### Key import rewrites

- `use crate::Strategy` → `use strategy::Strategy` in moved `strategy.rs`
- `strategy::llm_forecaster::*` → `trader::llm_forecaster::*` in all 10 test files + bin
- `crates/strategy::llm_forecaster` doc-comment → `crates/trader::llm_forecaster`
  in `crates/ui/src/assistant/state.rs`

### Gate outcomes

| Gate | Result |
|------|--------|
| `t1809_no_strategy_crate_consumes_reflection_retrieval` | GREEN (was RED) |
| `t1810_trader_crate_owns_reflection_retrieval` (new) | GREEN |
| `cargo build --workspace --all-targets` | GREEN |
| `scripts/verify_anchors.sh` | ANCHORS PASS (34/34) |
| Trader integration tests | 153 passed, 0 failed, 2 ignored |
| Strategy dep on reflection | REMOVED (cargo metadata confirms) |
| Cycle check (strategy→trader) | ABSENT |

## Changelog

- 2026-05-25 (analyst): authored v0.1.0 — recovery brief for R8.1
  violation introduced by v3-llm-forecaster Waves B/C/G. R1-R7 +
  K1-K8 + H1-H5 + Q1-Q7 + 10-item non-regression contract. Trace
  row REQ-REFLECTION-TRADER-001 opened at `proposed` state. HANDOFF
  → architect for M-T1.
- 2026-05-26 (developer): M-DEV complete per tasks.md T-D-N1..T-D-N12.
  All gates green. HANDOFF → tester for M-FINAL.
