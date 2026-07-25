# Story 1.8: v5-latency-slippage-sim

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want deterministic latency & slippage simulation closing the backtest-vs-live gap (canonical medium-friction model, slippage_bps 8, square-root market impact), landed across the v0.1-v0.5 chain,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

## Acceptance Criteria

1. **Given** the repo history at `v5-latency-slippage-sim`'s landing commits (`git log -- spec/v5-latency-slippage-sim`), **when** the recorded verification for `v5-latency-slippage-sim` is replayed (tests, reports under `spec/v5-latency-slippage-sim/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: deterministic latency & slippage simulation closing the backtest-vs-live gap (canonical medium-friction model, slippage_bps 8, square-root market impact), landed across the v0.1-v0.5 chain.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `v5-latency-slippage-sim` 0.1.0 - the base feature (shipped)
- [x] Folded iteration `v5-latency-slippage-sim-v0.2.0-anchor-migration` (0.1.0, frontmatter `shipped`): v5 latency-slippage-sim v0.2.0 — anchor migration to canonical non-zero friction - carries `REQ-V5-ANCHOR-MIGRATION-V0-2-0-001` (provenance: `git log -- spec/v5-latency-slippage-sim-v0.2.0-anchor-migration`)
- [x] Folded iteration `v5-latency-slippage-sim-v0.3.0-full-path-wiring` (0.3.0, frontmatter `shipped`): v5 latency-slippage-sim v0.3.0 — full-path wiring + Group A data-source decision + t1937 fix - carries `REQ-V5-FULL-PATH-WIRING-001` (provenance: `git log -- spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring`)
- [x] Folded iteration `v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit` (0.1.0, frontmatter `shipped`): v5 latency-slippage-sim v0.4.0 — candle/realdata feature-gated re-emit - carries `REQ-V5-LATENCY-SLIPPAGE-V0-4-0-001` (provenance: `git log -- spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit`)
- [x] Folded iteration `v5-latency-slippage-sim-v0.5.0-square-root-market-impact` (0.2.0, frontmatter `shipped`): v5 latency-slippage-sim v0.5.0 — square-root market-impact model - carries `REQ-V5-LATENCY-SLIPPAGE-V0-5-0-001` (provenance: `git log -- spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact`)

## Dev Notes

- Source feature folder: `spec/v5-latency-slippage-sim/` - frontmatter status **`shipped`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Strategy — `**v5**` (deterministic latency & slippage sim, v0.1→v0.5 chain).
- Provenance: `git log -- spec/v5-latency-slippage-sim` (full narrative); reports under `spec/v5-latency-slippage-sim/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-V5-LATENCY-SLIPPAGE-001` (state=`shipped`) · `REQ-V5-ANCHOR-MIGRATION-V0-2-0-001` (state=`shipped`) · `REQ-V5-FULL-PATH-WIRING-001` (state=`shipped`) · `REQ-V5-LATENCY-SLIPPAGE-V0-4-0-001` (state=`shipped`) · `REQ-V5-LATENCY-SLIPPAGE-V0-5-0-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
