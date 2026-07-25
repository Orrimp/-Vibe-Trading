---
adr: 0074
title: Read-only reflection decision-support surface at the advisor decision points — trader-layer read helper + core-typed UI summary mirror
status: accepted
date: 2026-06-26
supersedes: none
superseded-by: none
extends: 0041, 0064
---

# ADR-0074 — Read-only reflection decision-support surface at the advisor decision points

## Context

`spec/product.md` names a **deterministic learning loop** as core pillar 3.
The honest scope (analyst, `spec/advisor-reflection-decision-loop/feature.md`,
operator-approved 2026-06-26): C4-as-autonomous-param/route-loop is **moot on a
ship-passive product and a pre-registration footgun**; the genuinely-useful,
highest-integrity slice is a **deterministic decision-support memory surface** —
read the already-built reflection store at the advisor's decision points and tell
the operator, factually and past-only, *"you've paper-traded this coin/strategy
before; here's what the gate said."* The operator is the adapter; memory
**surfaces** prior evidence, never *re-ranks* the bake-off (the F9 / ADR-0064
narration-only treatment, applied to memory).

The reflection loop is ~90% built. The WRITE side is wired end-to-end
(`crates/exec/src/paper.rs::on_trade_close:123` → `ReflectionWriterTask::run`
`crates/reflection/src/writer/task.rs:38` → `post_mortem_analyst::generate_card`
→ `store.upsert`); the READ side is wired for the standalone Memory browser
(`crates/ui/src/bin/cockpit_live.rs:872` → `reflection::query::open_and_list_recent`
→ `LessonCardCard` → `Message::MemoryHydrate`). The gap is that the three
advisor **decision** screens — `crates/ui/src/screens/leaderboard.rs`,
`tune.rs`, `forward_plan.rs` — carry **zero** reflection reads; the Memory screen
is disconnected from the decision moment.

Two layering constraints already exist and bind this work:

1. **ADR-0041** (`0041-trader-crate-split.md`) forbids `crates/strategy` from
   consuming reflection-retrieval and homes the consumer in `crates/trader`.
   The defensive gate `crates/reflection/tests/no_strategy_caller.rs` enforces
   it: `t1809` fails CI if any `crates/strategy/src/**` file references
   `reflection::retrieve_top_k` / `reflection::store::` / `reflection::ReflectionStore`;
   `t1810` asserts at least one `crates/trader/src/**` file keeps a
   `reflection::retrieve_top_k` consumer.
2. **ADR-0064** (`0064-advisor-llm-narration-seam.md`) established the
   narration-only, triggered-second-async-step, `core`-typed-ui-mirror pattern
   on the Leaderboard. This ADR reuses that pattern's seam discipline for memory
   instead of LLM prose.

The existing trader-layer reflection consumer is `ForecastContext::from_runtime`
(`crates/trader/src/llm_forecaster/types.rs:496`, calls `retrieve_top_k` at
`:516`) — it satisfies `t1810` today. The new read helper sits alongside it in
`crates/trader`, so `t1810` is doubly satisfied and `t1809` is structurally
unaffected (no `strategy` edit).

## Decision

### D1 — A new read-only helper module `crates/trader/src/decision_memory.rs`

A read-only helper, homed in `crates/trader` (the ADR-0041-sanctioned reflection
consumer — NOT `strategy`, NOT `ui`), answers at a decision point: "for THIS coin
(+ optionally this strategy), what past **forward paper-trade** lessons exist, and
what was the outcome / regime?" Signature (async because `ReflectionStore::top_k`
is async, the `from_runtime` precedent):

```rust
pub async fn recall_decision_lessons(
    store: &dyn reflection::ReflectionStore,
    query: &DecisionMemoryQuery,
) -> Result<DecisionMemorySummary, reflection::RetrievalError>;
```

`DecisionMemoryQuery` is a thin trader-owned input — `symbol: Symbol`,
`strategy: Option<StrategyId>`, `current_regime: reflection::RegimeTag`, `k: usize`
— that the helper turns into a `reflection::RetrievalQuery` (the helper supplies
a sentinel `strategy_id` of `(unattributed)` when `strategy == None`, mirroring
`build_retrieval_query`'s symbol-fallback discipline,
`crates/reports/src/render/memory_highlights.rs:139`). The helper runs
`retrieve_top_k(store, &q, k)` (relevance) and reduces the returned
`Vec<reflection::LessonCard>` into the **core-typed** `DecisionMemorySummary` —
the helper is the **only** place a `reflection::LessonCard` is read on the
decision path. Read-only: it NEVER writes, NEVER feeds back into signal
generation, the bake-off ranking, or `rank_candidates`. Fail-soft is inherited
from the store (an empty / absent store yields `Ok(vec![])` →
`DecisionMemorySummary::empty()`).

### D2 — The `core`-typed `DecisionMemorySummary` boundary (no reflection type crosses into `ui`)

`recall_decision_lessons` returns a `DecisionMemorySummary` built from
`trading_core` + `std` types **only** — NO `reflection::LessonCard`,
`OutcomeClass`, or `RegimeTag` crosses into the carrier. This mirrors the Memory
screen's existing `reflection → ui` boundary (`LessonCardCard` is deliberately
distinct from `reflection::LessonCard`, `crates/ui/src/memory/state.rs:39`), and
keeps `crates/ui` free of a reflection / sqlx dep edge. Shape:

```rust
pub struct DecisionMemorySummary {
    pub symbol: Symbol,
    pub strategy: Option<StrategyId>,
    pub match_count: usize,                  // how many lessons matched
    pub most_recent: Option<DecisionMemoryEntry>, // the headline (latest by closed_at)
    pub entries: Vec<DecisionMemoryEntry>,   // top-k, ranked, for richer surfaces
}
pub struct DecisionMemoryEntry {
    pub strategy: StrategyId,
    pub outcome: DecisionOutcome,            // trader-owned: Win | Loss | Scratch
    pub signed_pnl: Money<Usdt>,
    pub regime: DecisionRegime,              // trader-owned: Bull|Bear|Chop|Volatile|Calm
    pub closed_at: Timestamp,
}
```

`DecisionOutcome` / `DecisionRegime` are **trader-owned closed enums** mapped
one-for-one from `reflection::OutcomeClass` / `reflection::RegimeTag` inside the
helper (the closed-enum-mirror discipline ADR-0064 § D4 uses for
`RecommendationOutcome`). The `Money<Usdt>` + `Timestamp` are already `core`
types and pass through. `DecisionMemorySummary::is_empty()` ⇔ `match_count == 0`;
the empty case is the dominant fresh-workstation path and is a first-class honest
state, NOT an error.

### D3 — Three decision-point surfaces, each a `core`-typed → `ui` mirror + a `…MemoryHydrate` message (the F9 seam)

The three UI surfaces consume an **already-mapped, `ui`-owned** struct via a
per-screen hydrate message, mirroring the Memory-screen boot-hydrate
(`Message::MemoryHydrate`, `cockpit_live.rs:872-921`) and the F9 narration
lifecycle (`NarrationState` / `BakeoffNarration{Requested,Completed}`,
`crates/ui/src/leaderboard/state.rs:149`). NO `reflection` type and NO
`trader::DecisionMemorySummary` crosses `view` — each screen carries a
`ui`-owned `MemoryNote` (a `SmolStr`-only render-model) on its screen-state,
populated from a `Message::<Screen>MemoryHydrate(MemoryNote)` arm:

- **S1 Leaderboard** (`LeaderboardScreenState`): a `memory_note: MemoryNoteState`
  field keyed on the chosen coin. When a bake-off renders for `(coin, regime)`,
  the binary fires a hydrate task that runs `recall_decision_lessons` and maps
  the summary → `MemoryNote` → `Message::LeaderboardMemoryHydrate`. Renders a
  small factual chip near the recommendation block: *"You've paper-traded
  `macd_trend` on XRPUSDT before — last forward run closed LOSS (−$X) in a bear
  regime."* `match_count == 0` ⇒ the chip does NOT render (silent absence; Q5).
- **S2 Tune** (`TuneScreenState`): a `memory_note: MemoryNoteState` keyed on
  `(coin, strategy-family)`. When the Tune editor opens for a family, hydrate and
  render a note: *"Last time you forward-ran a tuned `macd_trend` on XRPUSDT it
  closed FRAGILE-arm LOSS."* (the "FRAGILE-arm" framing is derived from the
  lesson's `outcome` + the family, NOT recomputed). Empty ⇒ no note.
- **S3 forward-plan** (`ForwardPlanScreenState`): a `memory_note: MemoryNoteState`
  for the crowned/promoted strategy on this coin — past-outcome context for the
  exact stance about to run forward. Lowest priority; the developer MAY defer S3
  to a v0.2 if budget tightens (the helper + S1/S2 stand alone).

The hydrate is a **pure sqlite read** (no provider, no network) — so unlike F9
(which needs an agent channel for the LLM), the memory surfaces use the simpler
`iced::Task::perform` + side-thread-tokio-`spawn` shape the Memory hydrate
already uses, returning the mapped `MemoryNote` via the per-screen message.

### D4 — The summary → `MemoryNote` mapping lives at one `#[cfg(feature = "live")]` boundary per screen

The single place a `trader::DecisionMemorySummary` is read on the `ui` side is a
`#[cfg(feature = "live")]` adapter (the `forward_plan/adapter.rs` precedent,
ADR-0062 § D4 / ADR-0064 § D4) — `MemoryNote::from_summary(&DecisionMemorySummary)`.
`trader` is an optional `ui` dependency (only `cockpit_live`'s `live` feature
pulls it, exactly as `agent` is for the forward-plan adapter), so the `ui` lib's
default build never names `trader` and `cargo tree -p ui` gains NO new edge.
Headless render tests construct `MemoryNote` directly via a `ui::fixtures`
helper — no `trader` / `reflection` type involved. If a `DecisionMemorySummary`
field drifts, this one adapter per screen is the only edit site (the mirror
discipline keeps the blast radius to one function).

### D5 — Recording scope stays forward-only for v1 (Q3 = a, default OFF)

v1 does NOT add a bake-off → lesson write tap. Lessons continue to come ONLY from
forward paper-trade closes (`crates/exec/src/paper.rs::on_trade_close:123`); the
bake-off path (`crates/backtest/src/bakeoff`, `crates/agent/src/plan.rs`) writes
no lessons (verified: zero reflection touch). The surface therefore honestly says
"you've **paper-traded** this before," NEVER "you've **backtested** this before";
a fresh operator who has only run bake-offs sees no memory notes — correct, not a
bug. Rationale: (a) it keeps the anchor surface trivially clean (no new write
path, no `OutcomeClass` semantics call for a never-realized backtest "trade", no
ADR-0041 *write*-seam decision); (b) recording every backtest as a "lesson"
conflates a hypothetical curve with a realized outcome and risks the surface
implying backtest results are evidence of forward edge — exactly the conflation
the ship-passive / pre-registration discipline guards against. A bake-off
write-tap is a clean, separately-justified v0.2 (it would need its own seam +
outcome-semantics + anchor scoping); if pursued it MUST write only to the
reflection sqlite store, never to a `spec/*/reports/` anchored file.

### D6 — Anchor-safe by construction + the frozen gate stays frozen

v1 is READ-ONLY at the decision points. It touches no `write_report` path; the
bake-off / backtest scenario emitters that produce the 119 anchored reports are
not modified (the helper sits strictly downstream; the surfaces are pure UI
adds). `classify_verdict` / `compute_robustness_flag` / `verdict_bands` /
`rank_candidates` + the ADR-0066 benchmark exemption are BYTE-UNCHANGED;
`BenchmarkWins` / `AllFragile` reachability is UNCHANGED (the surface is strictly
downstream of `rank_candidates`, exactly like F9). `bash scripts/verify_anchors.sh`
→ 119/119 before and after (run keyed by anchor NAME, not filename, per the
repo's anchors discipline). No `anchors.toml` SHA / `REVISION.toml` /
`spec/*/reports/` body touched → **no anchor-mutation ADR triggered**.

### D7 — Verification = render-PIXEL proofs + helper unit tests + the layering gate

The CLAUDE.md day-1 baseline-equity-divergence e2e is **N/A** — the surface
narrates past lessons, produces NO equity / signal / fill (like F6 forward-plan
and F9 narration; UNLIKE F4 sizing + F8 ensemble, where the strategy decision
variable bit the equity curve). The mandatory verification:

- **Render-PIXEL proofs** (`#![cfg(target_os = "macos")]`, ADR-0057 § D2,
  cosmic-text font-mutex serialized per `docs/dev-notes/iced-ui-render-verification.md`):
  for S1 (and S2) a populated-store fixture paints the memory chip / note as
  foreground in a scoped band (the `leaderboard_narration_render.rs` band-and-
  predicate template), with an **empty-store NEGATIVE control** asserting the
  band paints ~none of the chip (silent absence, NOT a broken panel — the rest
  of the screen still draws) and an anti-tautology discriminator (populated
  strictly exceeds empty in the chip band). Operator-facing PNGs saved to `/tmp`.
- **Helper unit tests** (deterministic, no UI): `recall_decision_lessons` over a
  `SqliteReflectionStore` / `NullReflectionStore` fixture — a populated store
  returns a non-empty summary with the correct headline (latest by `closed_at`,
  the tie-break the store guarantees) + the `OutcomeClass`→`DecisionOutcome` /
  `RegimeTag`→`DecisionRegime` mapping; an empty / absent store returns
  `DecisionMemorySummary::empty()` (`match_count == 0`).
- **Layering gate**: `t1809` stays green (no `strategy` edit), `t1810` stays
  green (the helper is a second `crates/trader` `retrieve_top_k` consumer); no
  `reflection::` type reaches a `view` fn (`grep -r 'reflection::'
  crates/ui/src/{screens,state,shell}` stays empty) + `cargo tree -p ui` gains
  NO new edge.

## Consequences

- The honest C4 ships as decision-support memory: the operator sees, at the
  bake-off / Tune / forward-plan decision moment, the factual past-only outcome
  of any coin/strategy they have forward-paper-traded. The operator is the
  adapter; the gate, the crown, and the ranking are untouched.
- One new read-only helper + one `core`-typed summary + three thin UI surfaces.
  No new crate, no new dependency, no `ui` dep-graph change, no anchor change.
- The surface is silent (renders nothing) for any coin not yet forward-run —
  the expected, correct, dominant state on a fresh workstation.
- A future bake-off → lesson write tap (Q3 = b) is a clean v0.2 with its own
  justification + outcome-semantics + anchor scoping; v1 does not pre-commit to
  it and is not blocked by it.

## Alternatives considered

- **Autonomous param/route adaptation (literal C4).** Rejected on integrity
  grounds (analyst § The honest tension): moot while the live field is
  all-Fragile; mutating a knob on a gate-rejected arm manufactures motion, not
  edge; a closed-loop param suggester feeding "robust-tuned" configs as future
  defaults is a silent gate-bypass. Recorded as an explicitly out-of-scope,
  gated follow-on, NOT v1.
- **Helper in `crates/strategy`.** Forbidden by ADR-0041 + `t1809`. Structurally
  impossible (strategy has no reflection path-dep).
- **Helper in `crates/ui`.** Rejected — `ui` has no sqlx / reflection dep and
  must keep its `LessonCardCard` / `MemoryNote` boundary; a reflection read in
  `ui` would invert the established `reflection → trader → message → ui` seam.
- **Reuse the Memory screen's `open_and_list_recent` directly from the decision
  screens.** Rejected — that is recency-only (no `(symbol, strategy, regime)`
  relevance filter) and would re-introduce a sqlite read into `ui`. The trader
  helper uses `retrieve_top_k` (cosine relevance) which is what a
  "have-I-tested-this-coin/strategy-before" surface needs.
- **Record bake-off backtests as lessons in v1 (Q3 = b).** Deferred to v0.2 —
  needs a write seam + outcome semantics + anchor scoping, and conflates a
  hypothetical curve with a realized outcome (D5).

## References

- Feature: `spec/advisor-reflection-decision-loop/feature.md`
- Extends: ADR-0041 (trader-crate split / reflection-consumer home),
  ADR-0064 (narration-only seam + `core`-typed-ui mirror + triggered async step)
- Leans on: ADR-0062 (one-adapter `core`-typed-ui mirror), ADR-0057 § D2
  (macOS render-pixel canonicality), ADR-0066 (benchmark exemption — reachability
  unchanged)
- Code seams: `crates/reflection/src/retrieval.rs:22` (`retrieve_top_k`),
  `crates/reflection/src/query.rs:41` (`open_and_list_recent`),
  `crates/reflection/src/types.rs:79,108` (`LessonCard` / `RetrievalQuery`),
  `crates/trader/src/llm_forecaster/types.rs:496` (`from_runtime`, the consumer
  precedent), `crates/reflection/tests/no_strategy_caller.rs` (t1809/t1810),
  `crates/ui/src/memory/state.rs:39` (`LessonCardCard` boundary),
  `crates/ui/src/bin/cockpit_live.rs:872` (Memory hydrate pattern),
  `crates/ui/src/leaderboard/state.rs:149` (`NarrationState` F9 lifecycle),
  `crates/ui/src/forward_plan/adapter.rs` (the `#[cfg(feature = "live")]` adapter
  precedent)

## Changelog

- 2026-06-26 (architect): authored. Read-only reflection decision-support surface
  — trader-layer `recall_decision_lessons` helper (ADR-0041 home, alongside the
  `from_runtime` consumer) + a `core`-typed `DecisionMemorySummary` boundary
  (no reflection type crosses into `ui`, the `LessonCardCard` discipline) + three
  decision-point surfaces (S1 Leaderboard chip, S2 Tune note, S3 forward-plan
  context) each a `ui`-owned `MemoryNote` hydrated via a per-screen
  `…MemoryHydrate` message (the Memory-hydrate + F9-narration seam, simplified to
  a pure sqlite read). Recording stays forward-only (Q3 = a). Anchor-safe by
  construction (read-only, no `write_report` path → 119/119); frozen gate +
  ranking + `BenchmarkWins`/`AllFragile` reachability UNCHANGED. Render-PIXEL
  verified with a populated fixture + empty-store negative control.
