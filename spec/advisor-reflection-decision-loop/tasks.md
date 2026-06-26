---
slug: advisor-reflection-decision-loop
status: in-progress
owner: architect → developer
updated: 2026-06-26
version: 0.1.0
---

# Tasks — C4 read-only reflection decision-support surface

Design: `feature.md § Design` + ADR-0074
(`spec/architecture/adr/0074-reflection-decision-surface.md`).
Sequenced so each task compiles + tests green before the next. **Read-only at
the decision points; frozen gate; helper in `crates/trader` per ADR-0041.**

Legend: `[ ]` todo · all tasks are additive; no anchored content is touched.

## Phase A — the trader-layer read helper (the foundation)

- [ ] **T1 — `recall_decision_lessons` helper.** NEW
  `crates/trader/src/decision_memory.rs`. `pub async fn
  recall_decision_lessons(store: &dyn reflection::ReflectionStore, query:
  &DecisionMemoryQuery) -> Result<DecisionMemorySummary,
  reflection::RetrievalError>`. Builds a `reflection::RetrievalQuery`
  (`crates/reflection/src/types.rs:108`) from `DecisionMemoryQuery` — sentinel
  `StrategyId::new("(unattributed)")` when `strategy == None` (the
  `build_retrieval_query` symbol-fallback discipline,
  `crates/reports/src/render/memory_highlights.rs:139`) — and calls
  `reflection::retrieve_top_k(store, &q, k)` (`crates/reflection/src/retrieval.rs:22`),
  `k` defaulting to `reflection::REPORT_TIME_TOP_K` (=5, `lib.rs:69`). Async,
  mirroring `ForecastContext::from_runtime` (`crates/trader/src/llm_forecaster/types.rs:496`).
  Read-only — no write, no feedback into ranking/signal generation.
- [ ] **T2 — `DecisionMemoryQuery` input type.** `symbol: Symbol`, `strategy:
  Option<StrategyId>`, `current_regime: reflection::RegimeTag`, `k: usize`.
  Trader-owned; `trading_core` + `reflection::RegimeTag` only.
- [ ] **T3 — `t1810` / `t1809` stay green.** Confirm
  `crates/trader/src/decision_memory.rs` contains `reflection::retrieve_top_k`
  (so `t1810` holds even if `from_runtime` is ever refactored) AND that NO
  `crates/strategy/src/**` file gained a reflection reference (`t1809`). Run
  `cargo test -p reflection --test no_strategy_caller`.
- [ ] **T4 — helper unit tests** (deterministic, no UI). Build a
  `SqliteReflectionStore` fixture (the `crates/reflection/src/query.rs`
  `build_test_pool` pattern — migrate + insert lesson cards): a populated store
  returns a non-empty summary with the correct headline (latest by `closed_at`,
  the store's tie-break `crates/reflection/tests/store_top_k_determinism.rs`) +
  correct `OutcomeClass`→`DecisionOutcome` / `RegimeTag`→`DecisionRegime`
  mapping; a `NullReflectionStore` (`crates/reflection/src/store/`) AND an
  empty/absent store both return `DecisionMemorySummary::empty()`
  (`match_count == 0`).

## Phase B — the `core`-typed UI summary boundary

- [ ] **T5 — `DecisionMemorySummary` + `DecisionMemoryEntry`.** Built from
  `trading_core` + `std` only (NO `reflection::LessonCard` / `OutcomeClass` /
  `RegimeTag`) — the `LessonCardCard` discipline (`crates/ui/src/memory/state.rs:39`).
  `DecisionMemorySummary { symbol, strategy, match_count, most_recent:
  Option<DecisionMemoryEntry>, entries: Vec<DecisionMemoryEntry> }`;
  `DecisionMemoryEntry { strategy, outcome: DecisionOutcome, signed_pnl:
  Money<Usdt>, regime: DecisionRegime, closed_at: Timestamp }`. `::empty()` +
  `::is_empty()`.
- [ ] **T6 — `DecisionOutcome` / `DecisionRegime` closed enums + mapping.**
  Trader-owned, mapped one-for-one from `reflection::OutcomeClass`
  (`outcome.rs:16`) / `reflection::RegimeTag` (`regime.rs:37`) inside the helper
  (the `RecommendationOutcome` closed-mirror discipline, ADR-0064 § D4,
  `crates/ui/src/leaderboard/state.rs:88`). Exhaustive `match` (a new
  reflection variant fails to compile until mapped). Re-export the public
  surface from `crates/trader/src/lib.rs`.

## Phase C — S1 Leaderboard memory chip (primary surface)

- [ ] **T7 — `MemoryNoteState` + `MemoryNote` (ui-owned render-model).**
  `SmolStr`-only. `MemoryNoteState { Absent, Present(MemoryNote) }`
  (default `Absent`); `MemoryNote { headline: SmolStr, match_count: usize }`.
  Homed where the leaderboard render-models live (`crates/ui/src/leaderboard/state.rs`),
  re-usable by S2/S3.
- [ ] **T8 — S1 state field + transitions.** Add `memory_note: MemoryNoteState`
  to `LeaderboardScreenState` (`crates/ui/src/leaderboard/state.rs:593`, the
  three-touchpoint field + `Debug` + `Default` pattern; default `Absent`). A
  `set_memory_note(MemoryNoteState)` setter + a reset in `begin_run` (so a prior
  coin's note never lingers, mirroring `begin_run`'s narration reset at `:692`).
- [ ] **T9 — `Message::LeaderboardMemoryHydrate(MemoryNoteState)` + pure update
  arm.** In `crates/ui/src/state.rs` (the `MemoryHydrate` precedent at
  `:2123`). The update arm is pure (sets the field); no I/O in `update`.
- [ ] **T10 — `MemoryNote::from_summary` adapter** (`#[cfg(feature = "live")]`).
  The ONE place `trader::DecisionMemorySummary` is read on the `ui` side — the
  `crates/ui/src/forward_plan/adapter.rs` precedent. Pre-formats the headline
  (outcome, signed P&L, regime, strategy) into the `MemoryNote.headline`
  `SmolStr`; `summary.is_empty()` → `MemoryNoteState::Absent`. Copy via
  `crate::strings` (zero literals), including the "informational memory, not a
  recommendation" label + the not-advice/past-performance disclaimer.
- [ ] **T11 — the hydrate `iced::Task`** in `crates/ui/src/bin/cockpit_live.rs`
  (`#[cfg(feature = "live")]`), mirroring the Memory hydrate (`:872-921`):
  `iced::Task::perform` + side-thread-tokio `spawn` calling
  `trader::recall_decision_lessons(store, {coin, None, regime, k})` →
  `MemoryNote::from_summary` → `Message::LeaderboardMemoryHydrate`. Fired when a
  bake-off renders for `(coin, regime)`. Pure sqlite read — no provider, no
  network. Fail-soft: any error → `MemoryNoteState::Absent` (+ `warn!`).
- [ ] **T12 — S1 chip render** in `crates/ui/src/screens/leaderboard.rs`. Render
  the chip near the recommendation block ONLY when `MemoryNoteState::Present`;
  `Absent` renders nothing (Q5). No new theme token, no new widget (the F9
  narration-section discipline). Annotates only — never reorders the table,
  never touches the crown/gate.

## Phase D — S2 Tune memory note

- [ ] **T13 — S2 state + message.** `memory_note: MemoryNoteState` on
  `TuneScreenState` (`crates/ui/src/tune/screen_state.rs`);
  `Message::TuneMemoryHydrate(MemoryNoteState)` + pure update arm.
- [ ] **T14 — S2 hydrate + render.** Hydrate `iced::Task` (the T11 shape) with
  `strategy = Some(<family id>)` keyed on `(coin, family)`, fired when the Tune
  editor opens; render the note (past-only outcome wording derived from the
  lesson, NOT a fresh gate run). `Absent` ⇒ no note. Reuses `MemoryNote::from_summary`.

## Phase E — S3 forward-plan memory context (optional v1; deferrable to v0.2)

- [ ] **T15 — S3 state + message** (optional). `memory_note: MemoryNoteState` on
  `ForwardPlanScreenState` (`crates/ui/src/forward_plan/state.rs`);
  `Message::ForwardPlanMemoryHydrate(MemoryNoteState)` + pure update arm.
- [ ] **T16 — S3 hydrate + render** (optional). Past-outcome context for the
  crowned/promoted strategy on this coin; the T11 hydrate shape keyed on
  `(coin, crowned-strategy)`. If budget tightens, defer T15/T16 (+ T19) to a
  v0.2 of this feature — the helper + S1/S2 stand alone.

## Phase F — render-PIXEL proofs (CLAUDE.md: verify at the rendered-pixel layer)

- [ ] **T17 — S1 render-PIXEL proof.** NEW
  `crates/ui/tests/leaderboard_memory_chip_render.rs` (`#![cfg(target_os =
  "macos")]`, ADR-0057 § D2; cosmic-text font-mutex serialized per
  `spec/dev-notes/iced-ui-render-verification.md`), modelled on
  `crates/ui/tests/leaderboard_narration_render.rs`: (a) a **populated-store
  fixture** (`MemoryNoteState::Present`) paints the chip as foreground in a
  scoped band near the recommendation block; (b) an **empty-store NEGATIVE
  control** (`MemoryNoteState::Absent`) — the chip band paints ~none of the chip
  AND the rest of the leaderboard still draws (silent absence, NOT a broken
  panel); (c) an **anti-tautology discriminator** (populated strictly exceeds
  empty in the chip band). Operator-facing PNGs to `/tmp`. Add the populated +
  empty `MemoryNote` fixtures to `crates/ui/src/fixtures.rs` +
  `crates/ui/src/test_support.rs` (constructed directly — no `trader`/`reflection`
  type).
- [ ] **T18 — S2 render-PIXEL proof.** NEW `crates/ui/tests/tune_memory_note_render.rs`
  (same populated + empty-control + discriminator shape, scoped to the Tune note band).
- [ ] **T19 — S3 render-PIXEL proof** (optional, ships iff S3 ships). NEW
  `crates/ui/tests/forward_plan_memory_render.rs`.

## Phase G — recording scope + anchors + close

- [ ] **T20 — recording stays forward-only (Q3 = a).** Confirm no bake-off →
  lesson write tap is added: `crates/backtest/src/bakeoff` +
  `crates/agent/src/plan.rs` keep zero reflection writes; lessons come only from
  `crates/exec/src/paper.rs::on_trade_close:123`. (Bake-off write-tap is a
  v0.2 — out of v1 scope.)
- [ ] **T21 — `ui` dep-graph + no-reflection-in-`view` guards.** `cargo tree -p
  ui` UNCHANGED (the `DecisionMemorySummary → MemoryNote` map is
  `#[cfg(feature = "live")]` only; `MemoryNote` is `ui`-owned). `grep -r
  'reflection::' crates/ui/src/{screens,state,shell}` stays empty (the helper's
  `core`-typed summary + the `ui`-owned `MemoryNote` keep `reflection` out of `view`).
- [ ] **T22 — anchors 119/119.** `bash scripts/verify_anchors.sh` → ANCHORS
  PASS (119/119) before AND after (keyed by anchor NAME, not filename). No
  `write_report` path, `anchors.toml` SHA, `REVISION.toml`, or `spec/*/reports/`
  body touched. Frozen gate (`classify_verdict` / `compute_robustness_flag` /
  `verdict_bands` / `rank_candidates` + ADR-0066 benchmark exemption)
  byte-unchanged; `BenchmarkWins`/`AllFragile` reachability unchanged.
- [ ] **T23 — full gate.** `cargo fmt`, `cargo clippy --workspace -- -D
  warnings`, `cargo test --workspace` (incl. `no_strategy_caller` t1809/t1810
  + the new helper unit tests + the macOS render-pixel proofs). Day-1
  divergence e2e is **N/A** (narration-only — no equity/signal/fill, like F6/F9).

## Handoff notes for the developer

- **The seam to copy, end to end:** the Memory hydrate
  (`crates/ui/src/bin/cockpit_live.rs:872-921` → `LessonCardCard` →
  `Message::MemoryHydrate`) for the async sqlite read, and the F9 narration
  lifecycle (`crates/ui/src/leaderboard/state.rs:149` `NarrationState` + the
  `#[cfg(feature = "live")]` `forward_plan/adapter.rs` for the one engine-type →
  ui-type map). Memory is the simpler of the two — a pure sqlite read, no agent
  channel.
- **Non-negotiable layering:** the helper is `crates/trader` (ADR-0041 /
  `t1809`); the `ui` surfaces receive a `ui`-owned `MemoryNote` via a message;
  no `reflection::` type in `crates/ui/src/{screens,state,shell}`.
- **Empty-store is the dominant path** — the empty-store negative control (T17b)
  is the load-bearing proof, not an afterthought (the empty case must NOT leave a
  broken/empty panel).
