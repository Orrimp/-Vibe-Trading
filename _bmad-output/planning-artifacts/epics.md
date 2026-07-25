---
stepsCompleted: [1, 2, 3, 4]
inputDocuments:
  - _bmad-output/planning-artifacts/PRD.md
  - _bmad-output/planning-artifacts/PRD-addendum.md
  - _bmad-output/planning-artifacts/architecture.md
generationMode: retroactive-brownfield (BMAD migration Phase 2, 2026-07-25)
sourceOfTruthUntilCutover: spec/ (feature.md frontmatter + trace.toml + CHANGELOG.md)
---

# trading - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for **trading** (The Honest Advisor), decomposing the requirements from the PRD, and Architecture requirements into implementable stories.

> **Retroactive brownfield breakdown.** The product is shipped and feature-complete (2026-07-09; maintenance mode). These epics/stories are the **BMAD-native registry of already-delivered work**, generated 1:1 from the 155 `spec/**/feature.md` folders per the ratified migration plan (`spec/dev-notes/bmad-migration-plan-2026-07-24.md`, Phase 2): **141 top-level stories** (iteration folders in `spec_lint.py`'s `CHANGELOG_ROLLUP_ALLOWLIST` fold as sub-tasks of their base story) **+ 2 forward stories** without feature folders (the live BMAD migration; the `lab-recipe-test-harness` v0.3 backlog item). Story **titles are the feature slugs** so the sprint-status key equals the slug and the story <-> `trace.toml` `REQ-*` bijection stays mechanical (the Phase 5b lint re-founding keys on it). Statuses live in the story files and `sprint-status.yaml`, mapped honestly from `feature.md` frontmatter - never silently promoted.

## Requirements Inventory

### Functional Requirements

FR-1: Guided coin + budget + lookback input starts a bake-off from a single guided cockpit form
FR-2: Honest EUR->USDT budget conversion at a configurable static rate, displayed honestly; ranking FX-invariant
FR-3: All-arms bake-off with the always-present buy-and-hold benchmark arm; determinate progress; anchors untouched
FR-4: Timeframe (H1/H4/D1) and starting-capital controls; capital scales absolute equity, never the ranking
FR-5: On-demand market data for any coin + window into a git-ignored cache; graceful degradation
FR-6: Robustness-gated ranking (D1): FRAGILE shown never crowned; benchmark exempt; BenchmarkWins the modal honest outcome
FR-7: Leaderboard transparency: Sharpe/return/drawdown + Churn + tail-risk block + display-only data-quality panel
FR-8: Report-only overfitting scorecard (N_eff -> DSR -> MinBTL) + crown-credibility weak-evidence band; never a veto
FR-9: Confidence-not-verdict framing + mandatory not-advice/past-performance/simulated-budget disclaimers
FR-10: Templated plain-language rationale for every outcome code, LLM-free
FR-11: Opt-in faithful LLM narration with a deterministic faithfulness post-check and templated fallback; never in the ranking
FR-12: Conditional rule-driven forward plan (D2): stance + plain-language rules + projected sizing, 1-30d horizon; not a forecast
FR-13: Deterministic plan export (plan-{coin}-{window}-{seed8}.md, golden-locked wording; no orders, no LLM at export)
FR-14: Forward-run fidelity: the actual crowned strategy runs forward (registry identity; typed error on unknown id; every arm forward-buildable)
FR-15: Hard budget cap + honest P/L display (day-1 divergence e2e; short disclaimer; opt-in lot realism, default byte-identical)
FR-16: Gate-tied parameter sweep: every config scored by the identical frozen gate; FRAGILE promote-lock; bounded grid
FR-17: Promotion of a surviving tuned config into plan + paper-run with the tuned-rules honesty header
FR-18: Pre-registered slates only - no search anywhere in the field-definition path; day-1 divergence e2e per new arm class
FR-19: Simulated directional shorts (paper-only, separate slate) with honest short mechanics and warnings
FR-20: The frozen robustness gate: pre-registered block-bootstrap Monte-Carlo rule, byte-frozen bands, active in the live bake-off
FR-21: Reproducibility, audit ledger & anchored evidence: recorded seed/window/KPIs; double-entry ledger; 119 byte-immutable anchors; PIT discipline

### NonFunctional Requirements

NFR-1: Determinism & reproducibility: same (coin, window, seed) -> same ranking; run-varying values never contaminate anchored bodies
NFR-2: Honesty surfaces: not-advice + past-performance + simulated-budget disclaimers everywhere; null results first-class
NFR-3: Auditability: every paper fill and recommendation reconstructible from the ledger + recorded run parameters
NFR-4: No-regression floor: verify_anchors 119/119 + full lib/integration/UI-snapshot suite green on every change
NFR-5: UI truthfulness at the pixel layer: rendered-output verification with populated states + negative controls
NFR-6: Graceful degradation: missing corpora skip arms; LLM failure falls back to templated copy; forward build errors never strand the Live view

### Additional Requirements

*From `architecture.md` - the 19 adopted invariants (AD-1..AD-19) are binding on all maintenance work:*

- AD-1 FROZEN robustness gate byte-frozen
- AD-2 anchors 119/119 byte-identical
- AD-3 anchor-safety by construction (write_report=false)
- AD-4 feature.md status is the lifecycle source of truth (story files mirror it until Phase 5b re-keys the triad to sprint-status)
- AD-5 PIT discipline structural + linted
- AD-6 buy-and-hold benchmark-exempt
- AD-7 narration faithfulness gate or fallback
- AD-8 additive-only; three registration seams; no plugin architecture
- AD-9 money is Decimal never f64
- AD-10 UI verified at the rendered-PIXEL layer
- AD-11 do-not-build register binding; thesis era-qualified
- AD-12 DSR report-only; crown-veto stays unbuilt
- AD-13 3-OS CI active; macOS canonical visual box
- AD-14 dependency-direction law
- AD-15 PAPER/SIM only - no live execution path
- AD-16 day-1 baseline-equity-divergence e2e for every overlay/sizing modifier
- AD-17 determinism envelope
- AD-18 ADR registry atomic
- AD-19 release discipline: gates green; REGRESSION blocks ship; no secrets in git

*Cross-cutting invariant rows in `spec/trace.toml` not bound to any single story (meta - carried by AD-9/AD-17):*
- `INV-DETERMINISM-RNG-001` (state=`shipped`)
- `INV-MONEY-DECIMAL-001` (state=`shipped`)
- `INV-TIMESTAMPS-FRACSEC-001` (state=`shipped`)

### UX Design Requirements

None - no bmad-ux design contract exists for this brownfield migration. UI requirements are carried by AD-10 (render-PIXEL verification), NFR-5, and the Epic 2 stories (the shipped Lumen design system + `spec/ui-design-principles.md`).

### FR Coverage Map

FR-1: Epic 3 - guided input (advisor-bakeoff-ranking F3 surface)
FR-2: Epic 3 - advisor-eur-fx
FR-3: Epic 3 - advisor-bakeoff-ranking (engine substrate: Epic 1)
FR-4: Epic 3 - leaderboard-timeframe-capital
FR-5: Epic 3 - advisor-dynamic-data
FR-6: Epic 3 - advisor-bakeoff-ranking + advisor-ensemble (gate activation) + advisor-benchmark-robustness
FR-7: Epic 4 - advisor-turnover-and-tail-metrics + advisor-data-quality-surface (leaderboard substrate: Epic 3)
FR-8: Epic 4 - advisor-overfitting-scorecard + advisor-no-alpha-gate-ci (band: Epic 5 advisor-crown-credibility)
FR-9: Epic 4 - advisor-confidence-not-verdict (disclaimers across Epics 3/5 surfaces)
FR-10: Epic 3 - advisor-llm-narration (templated path)
FR-11: Epic 3 - advisor-llm-narration (hardening: Epic 4 advisor-narration-faithfulness)
FR-12: Epic 3 - advisor-forward-plan
FR-13: Epic 5 - advisor-handoff-export
FR-14: Epic 3 - advisor-forward-paper (coverage completion: Epic 4 advisor-forward-fidelity-coverage)
FR-15: Epic 3 - advisor-forward-paper + advisor-short-selling display (lot realism: Epic 5 advisor-lot-realism)
FR-16: Epic 3 - advisor-param-tuning (stage promotion: Epic 5 advisor-calibrate-stage)
FR-17: Epic 3 - advisor-param-promotion
FR-18: Epic 3 - the pre-registered slates (combination/signal-library/DVOL/macro) - retired chains stay opt-in per Epic 7
FR-19: Epic 3 - advisor-short-selling
FR-20: Epic 1 - monte-carlo-bootstrap-path-generator + strategy-robustness-harness (live activation: Epic 3 advisor-ensemble)
FR-21: Epic 1 - v0-paper-sma ledger + anchors machinery (PIT: Epic 3 point-in-time-data-discipline; gates: Epic 6)

*(NFR-1..6 and AD-1..19 are cross-cutting: enforced by the Epic 6 gate/lint stories and the standing non-negotiables rather than owned by one story.)*

## Epic List

### Epic 1: Strategy & Backtest Engine (v0-v5 ladder + robustness program)
The pre-pivot engine: the v0-v5 strategy/backtest ladder, real-data corpora, and the concluded Monte-Carlo robustness program whose 2026-06-08 ship-passive verdict became the product's honesty core. Everything the advisor journey later reuses as its engine.
**FRs covered:** FR-20, FR-21
**Stories:** 23 (4 folded sub-task folder(s))

### Epic 2: Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates)
The iced cockpit: Lumen design-system shell, Live dashboards, the Lab run-save-compare loop, charts/tape/journal, and the render-PIXEL quality-gate harnesses that make UI claims provable.
**FRs covered:** cross-cutting (NFR-1..6, AD-1..19)
**Stories:** 63 (3 folded sub-task folder(s))

### Epic 3: Advisor MVP (F1-F9 + EUR-FX + dynamic data + PIT discipline)
The 2026-06-19 pivot product: the guided coin+budget journey - bake-off, robustness-gated ranking, forward plan, forward paper-trade - plus the pre-registered arm-class expansions and the point-in-time data primitive.
**FRs covered:** FR-1, FR-2, FR-3, FR-4, FR-5, FR-6, FR-10, FR-11, FR-12, FR-14, FR-15, FR-16, FR-17, FR-18, FR-19
**Stories:** 18 (0 folded sub-task folder(s))

### Epic 4: v2 Research-Driven Credibility Tranche
The 11-feature tranche distilled from the 900-paper research program: overfitting scorecard, churn/tail transparency, confidence-not-verdict framing, de-risk overlays, cost realism, narration faithfulness, the no-alpha CI capstone.
**FRs covered:** FR-7, FR-8, FR-9
**Stories:** 12 (1 folded sub-task folder(s))

### Epic 5: v3 "Prove It's Done" Close-Out
The bounded ship-readiness pass: Calibrate stage + journey stepper, corpus expansion + the P2 verdict re-run (the era-qualification evidence), and the P1/P3/P4/P5 remediation deliverables (crown-credibility band, PIT lint, lot realism, plan export).
**FRs covered:** FR-13
**Stories:** 6 (0 folded sub-task folder(s))

### Epic 6: Remediation, Infra & Governance (P0-P8, lints, BMAD migration)
Core infrastructure (reflection memory, operator reports, audit envelope, paper-soak) and the governance/tooling layer (schema/registry/staleness lints, anchors + spec-lint gates), plus cross-platform CI activation and the live BMAD-method migration.
**FRs covered:** cross-cutting (NFR-1..6, AD-1..19)
**Stories:** 10 (1 folded sub-task folder(s))

### Epic 7: Retired Research Lines (measured-and-retired bets)
The honest negative-result record: the v2.5 DL forecaster programme, the v3 forecaster/classifier bets, and the no-op-overlay precedent. Code + anchors retained; the lines are retired, and staying retired is a product guarantee (do-not-build register).
**FRs covered:** cross-cutting (NFR-1..6, AD-1..19)
**Stories:** 11 (5 folded sub-task folder(s))

## Epic 1: Strategy & Backtest Engine (v0-v5 ladder + robustness program)

The pre-pivot engine: the v0-v5 strategy/backtest ladder, real-data corpora, and the concluded Monte-Carlo robustness program whose 2026-06-08 ship-passive verdict became the product's honesty core. Everything the advisor journey later reuses as its engine.

### Story 1.1: v0-paper-sma

As the operator of the Honest Advisor,
I want the end-to-end paper-trading SMA-crossover tracer bullet (core types, Binance data, audit ledger, cockpit) proving a trivial strategy round-trips with reconciling double-entry books,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the repo history at `v0-paper-sma`'s landing commits (`git log -- spec/v1/v0-paper-sma`)
**When** the recorded verification for `v0-paper-sma` is replayed (tests, reports under `spec/v1/v0-paper-sma/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the end-to-end paper-trading SMA-crossover tracer bullet (core types, Binance data, audit ledger, cockpit) proving a trivial strategy round-trips with reconciling double-entry books
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.2: v05-composed-strategies

As the operator of the Honest Advisor,
I want hot-loadable TOML indicator/rule strategy assemblies (MACD + RSI + Bollinger) with atomic swap on file change,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the repo history at `v05-composed-strategies`'s landing commits (`git log -- spec/v1/v05-composed-strategies`)
**When** the recorded verification for `v05-composed-strategies` is replayed (tests, reports under `spec/v1/v05-composed-strategies/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: hot-loadable TOML indicator/rule strategy assemblies (MACD + RSI + Bollinger) with atomic swap on file change
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.3: v1-cross-sectional-momentum

As the operator of the Honest Advisor,
I want the cross-sectional top-N momentum strategy - the first multi-symbol real-edge candidate,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the repo history at `v1-cross-sectional-momentum`'s landing commits (`git log -- spec/v1/v1-cross-sectional-momentum`)
**When** the recorded verification for `v1-cross-sectional-momentum` is replayed (tests, reports under `spec/v1/v1-cross-sectional-momentum/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the cross-sectional top-N momentum strategy - the first multi-symbol real-edge candidate
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.4: v15a-mean-reversion-pairs

As the operator of the Honest Advisor,
I want mean-reversion on z-scored pairs, landing the pairs/portfolio plumbing,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the repo history at `v15a-mean-reversion-pairs`'s landing commits (`git log -- spec/v1/v15a-mean-reversion-pairs`)
**When** the recorded verification for `v15a-mean-reversion-pairs` is replayed (tests, reports under `spec/v1/v15a-mean-reversion-pairs/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: mean-reversion on z-scored pairs, landing the pairs/portfolio plumbing
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.5: v1-5b-multi-venue

As the operator of the Honest Advisor,
I want multi-venue support plus 1-second aggregated trades,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the repo history at `v1-5b-multi-venue`'s landing commits (`git log -- spec/v1/v1-5b-multi-venue`)
**When** the recorded verification for `v1-5b-multi-venue` is replayed (tests, reports under `spec/v1/v1-5b-multi-venue/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: multi-venue support plus 1-second aggregated trades
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.6: v2-llm-strategy

As the operator of the Honest Advisor,
I want the LLM news/sentiment strategy overlay - the first LLM-in-the-loop strategy, as support layer, not the alpha source,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the repo history at `v2-llm-strategy`'s landing commits (`git log -- spec/v1/v2-llm-strategy`)
**When** the recorded verification for `v2-llm-strategy` is replayed (tests, reports under `spec/v1/v2-llm-strategy/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the LLM news/sentiment strategy overlay - the first LLM-in-the-loop strategy, as support layer, not the alpha source
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.7: v2-1-tracing-layer-redactor

As the operator of the Honest Advisor,
I want a tracing-Layer secret redactor wired across all 17 binaries (REDACT_LAYER_MODE, WARN default),
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the repo history at `v2-1-tracing-layer-redactor`'s landing commits (`git log -- spec/v1/v2-1-tracing-layer-redactor`)
**When** the recorded verification for `v2-1-tracing-layer-redactor` is replayed (tests, reports under `spec/v1/v2-1-tracing-layer-redactor/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: a tracing-Layer secret redactor wired across all 17 binaries (REDACT_LAYER_MODE, WARN default)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.8: v5-latency-slippage-sim

As the operator of the Honest Advisor,
I want deterministic latency & slippage simulation closing the backtest-vs-live gap (canonical medium-friction model, slippage_bps 8, square-root market impact), landed across the v0.1-v0.5 chain,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the repo history at `v5-latency-slippage-sim`'s landing commits (`git log -- spec/v5-latency-slippage-sim`)
**When** the recorded verification for `v5-latency-slippage-sim` is replayed (tests, reports under `spec/v5-latency-slippage-sim/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: deterministic latency & slippage simulation closing the backtest-vs-live gap (canonical medium-friction model, slippage_bps 8, square-root market impact), landed across the v0.1-v0.5 chain
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.9: backtest-real-binance-data

As the operator of the Honest Advisor,
I want the real-Binance-data backtest path with a pinned corpus and deterministic seeds,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the repo history at `backtest-real-binance-data`'s landing commits (`git log -- spec/v1/backtest-real-binance-data`)
**When** the recorded verification for `backtest-real-binance-data` is replayed (tests, reports under `spec/v1/backtest-real-binance-data/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the real-Binance-data backtest path with a pinned corpus and deterministic seeds
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.10: simple-strategies-realdata

As the operator of the Honest Advisor,
I want sma / macd / rsi / bbands runnable on real Binance data in the Lab,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: sma / macd / rsi / bbands runnable on real Binance data in the Lab
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.11: binance-corpus-expansion

As the operator of the Honest Advisor,
I want the wider down-market Binance corpus (data/binance/, ADR-0032 pins) underpinning the bear surveys,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the repo history at `binance-corpus-expansion`'s landing commits (`git log -- spec/v1/binance-corpus-expansion`)
**When** the recorded verification for `binance-corpus-expansion` is replayed (tests, reports under `spec/v1/binance-corpus-expansion/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the wider down-market Binance corpus (data/binance/, ADR-0032 pins) underpinning the bear surveys
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.12: carry-funding-data-backfill

As the operator of the Honest Advisor,
I want the funding-rate data backfill feeding the carry strategy,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `dev-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: the funding-rate data backfill feeding the carry strategy
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.13: monte-carlo-bootstrap-path-generator

As the operator of the Honest Advisor,
I want the stationary-block-bootstrap path generator (Politis-White auto block length) that resamples real returns preserving fat tails and volatility clustering,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `tester-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: the stationary-block-bootstrap path generator (Politis-White auto block length) that resamples real returns preserving fat tails and volatility clustering
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.14: strategy-robustness-harness

As the operator of the Honest Advisor,
I want the distribution-summary backtest mode (Sharpe p5/p50/p95, drawdown tail, probability-of-loss) read against the frozen p5-Sharpe<0 -> FRAGILE rule,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `dev-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: the distribution-summary backtest mode (Sharpe p5/p50/p95, drawdown tail, probability-of-loss) read against the frozen p5-Sharpe<0 -> FRAGILE rule
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.15: momentum-parameter-robustness-sweep

As the operator of the Honest Advisor,
I want the distribution-per-theta sweep over the momentum family (verdict: FAMILY-UNIFORM-FRAGILE),
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `tester-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: the distribution-per-theta sweep over the momentum family (verdict: FAMILY-UNIFORM-FRAGILE)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.16: cross-sectional-mean-reversion-strategy

As the operator of the Honest Advisor,
I want the first pivot family through the robustness harness (verdict: FRAGILE),
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `tester-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: the first pivot family through the robustness harness (verdict: FRAGILE)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.17: time-series-momentum-robustness

As the operator of the Honest Advisor,
I want per-asset absolute momentum (long/flat) - the thesis-closing OHLCV test (verdict: FRAGILE),
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: per-asset absolute momentum (long/flat) - the thesis-closing OHLCV test (verdict: FRAGILE)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.18: horizon-retest-robustness

As the operator of the Honest Advisor,
I want the coarser 4h/daily decision-cadence retest - the last untested OHLCV axis (verdict: FRAGILE),
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: the coarser 4h/daily decision-cadence retest - the last untested OHLCV axis (verdict: FRAGILE)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.19: carry-strategy

As the operator of the Honest Advisor,
I want the cross-sectional funding-carry rotation (verdict: FRAGILE; retired),
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the repo history at `carry-strategy`'s landing commits (`git log -- spec/v1/carry-strategy`)
**When** the recorded verification for `carry-strategy` is replayed (tests, reports under `spec/v1/carry-strategy/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the cross-sectional funding-carry rotation (verdict: FRAGILE; retired)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.20: perp-basis-signal-robustness

As the operator of the Honest Advisor,
I want the perp-spot basis-reversal signal - the one LIVE/MEDIUM-HIGH certified signal, fee-gated to fragile in production,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: the perp-spot basis-reversal signal - the one LIVE/MEDIUM-HIGH certified signal, fee-gated to fragile in production
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.21: perp-basis-mn-spread

As the operator of the Honest Advisor,
I want the market-neutral basis spread study closing the derivatives-positioning channel (basis == funding byte-identically; negative median residual Sharpe),
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: the market-neutral basis spread study closing the derivatives-positioning channel (basis == funding byte-identically; negative median residual Sharpe)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.22: simple-strategy-overfit-guard

As the operator of the Honest Advisor,
I want the overfit/robustness guard proving the down-market trend "hedge" was 2-case noise,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the repo history at `simple-strategy-overfit-guard`'s landing commits (`git log -- spec/v1/simple-strategy-overfit-guard`)
**When** the recorded verification for `simple-strategy-overfit-guard` is replayed (tests, reports under `spec/v1/simple-strategy-overfit-guard/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the overfit/robustness guard proving the down-market trend "hedge" was 2-case noise
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 1.23: simple-strategy-bear-survey

As the operator of the Honest Advisor,
I want the two-stage bear-market survey over the 2021-22 corpus firming ship-passive on the deepest bear evidence,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

**Acceptance Criteria:**

**Given** the repo history at `simple-strategy-bear-survey`'s landing commits (`git log -- spec/v1/simple-strategy-bear-survey`)
**When** the recorded verification for `simple-strategy-bear-survey` is replayed (tests, reports under `spec/v1/simple-strategy-bear-survey/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the two-stage bear-market survey over the 2021-22 corpus firming ship-passive on the deepest bear evidence
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

## Epic 2: Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates)

The iced cockpit: Lumen design-system shell, Live dashboards, the Lab run-save-compare loop, charts/tape/journal, and the render-PIXEL quality-gate harnesses that make UI claims provable.

### Story 2.1: lumen-design-adoption

As the operator of the Honest Advisor,
I want the Lumen design-system master roadmap governing the multi-screen sidebar-shell migration (phases 1-5 shipped; phase 6 reserved),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the recorded brief in `spec/lumen-design-adoption/feature.md`
**When** the operator schedules the work (post do-not-build-register check)
**Then** the story delivers: the Lumen design-system master roadmap governing the multi-screen sidebar-shell migration (phases 1-5 shipped; phase 6 reserved)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.2: lumen-phase-1-foundation

As the operator of the Honest Advisor,
I want Lumen Phase 1: design tokens, chrome, and status bar foundation,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lumen-phase-1-foundation`'s landing commits (`git log -- spec/lumen-design-adoption/phase-1-foundation`)
**When** the recorded verification for `lumen-phase-1-foundation` is replayed (tests, reports under `spec/lumen-design-adoption/phase-1-foundation/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: Lumen Phase 1: design tokens, chrome, and status bar foundation
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.3: lumen-phase-2-shell-ia-charts

As the operator of the Honest Advisor,
I want Lumen Phase 2: shell, information architecture, and chart adoption,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lumen-phase-2-shell-ia-charts`'s landing commits (`git log -- spec/lumen-design-adoption/phase-2-shell-ia-charts`)
**When** the recorded verification for `lumen-phase-2-shell-ia-charts` is replayed (tests, reports under `spec/lumen-design-adoption/phase-2-shell-ia-charts/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: Lumen Phase 2: shell, information architecture, and chart adoption
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.4: lumen-phase-3-detail-screens

As the operator of the Honest Advisor,
I want Lumen Phase 3: detail-screen adoption,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lumen-phase-3-detail-screens`'s landing commits (`git log -- spec/lumen-design-adoption/phase-3-detail-screens`)
**When** the recorded verification for `lumen-phase-3-detail-screens` is replayed (tests, reports under `spec/lumen-design-adoption/phase-3-detail-screens/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: Lumen Phase 3: detail-screen adoption
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.5: lumen-phase-4-backtest-panel

As the operator of the Honest Advisor,
I want Lumen Phase 4: backtest-panel adoption,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lumen-phase-4-backtest-panel`'s landing commits (`git log -- spec/lumen-design-adoption/phase-4-backtest-panel`)
**When** the recorded verification for `lumen-phase-4-backtest-panel` is replayed (tests, reports under `spec/lumen-design-adoption/phase-4-backtest-panel/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: Lumen Phase 4: backtest-panel adoption
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.6: lumen-phase-5-humancontrol-agentfeed

As the operator of the Honest Advisor,
I want Lumen Phase 5: HumanControl + AgentFeed rename/adoption,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lumen-phase-5-humancontrol-agentfeed`'s landing commits (`git log -- spec/lumen-design-adoption/phase-5-humancontrol-agentfeed`)
**When** the recorded verification for `lumen-phase-5-humancontrol-agentfeed` is replayed (tests, reports under `spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: Lumen Phase 5: HumanControl + AgentFeed rename/adoption
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.7: lumen-phase-6-assistant-slot

As the operator of the Honest Advisor,
I want the reserved Lumen Phase 6 Assistant slot (forward-compat reservation only, gated on a v2 LLM assistant),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the recorded brief in `spec/lumen-design-adoption/phase-6-assistant-slot/feature.md`
**When** the operator schedules the work (post do-not-build-register check)
**Then** the story delivers: the reserved Lumen Phase 6 Assistant slot (forward-compat reservation only, gated on a v2 LLM assistant)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.8: ui-rethink-phase-a-lab

As the operator of the Honest Advisor,
I want the chart-centric Lab screen (UI rethink Phase A),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-rethink-phase-a-lab`'s landing commits (`git log -- spec/v1/ui-rethink-phase-a-lab`)
**When** the recorded verification for `ui-rethink-phase-a-lab` is replayed (tests, reports under `spec/v1/ui-rethink-phase-a-lab/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the chart-centric Lab screen (UI rethink Phase A)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.9: ui-rethink-phase-b-lab-run

As the operator of the Honest Advisor,
I want the Lab Run button + run plumbing (UI rethink Phase B),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-rethink-phase-b-lab-run`'s landing commits (`git log -- spec/v1/ui-rethink-phase-b-lab-run`)
**When** the recorded verification for `ui-rethink-phase-b-lab-run` is replayed (tests, reports under `spec/v1/ui-rethink-phase-b-lab-run/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the Lab Run button + run plumbing (UI rethink Phase B)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.10: ui-rethink-phase-c-sidebar-ia

As the operator of the Honest Advisor,
I want the sidebar IA flip + Live + strategy registry + settings rollup (UI rethink Phase C),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-rethink-phase-c-sidebar-ia`'s landing commits (`git log -- spec/v1/ui-rethink-phase-c-sidebar-ia`)
**When** the recorded verification for `ui-rethink-phase-c-sidebar-ia` is replayed (tests, reports under `spec/v1/ui-rethink-phase-c-sidebar-ia/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the sidebar IA flip + Live + strategy registry + settings rollup (UI rethink Phase C)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.11: ui-rethink-phase-d-trail

As the operator of the Honest Advisor,
I want the Trail (audit-journal) view (UI rethink Phase D, + follow-up patch),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-rethink-phase-d-trail`'s landing commits (`git log -- spec/v1/ui-rethink-phase-d-trail`)
**When** the recorded verification for `ui-rethink-phase-d-trail` is replayed (tests, reports under `spec/v1/ui-rethink-phase-d-trail/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the Trail (audit-journal) view (UI rethink Phase D, + follow-up patch)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.12: ui-rethink-phase-e-compare

As the operator of the Honest Advisor,
I want the Compare matrix screen (UI rethink Phase E),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-rethink-phase-e-compare`'s landing commits (`git log -- spec/v1/ui-rethink-phase-e-compare`)
**When** the recorded verification for `ui-rethink-phase-e-compare` is replayed (tests, reports under `spec/v1/ui-rethink-phase-e-compare/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the Compare matrix screen (UI rethink Phase E)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.13: ui-rethink-phase-f-memory-models-assistant

As the operator of the Honest Advisor,
I want the Memory + Models screens + the Phase-6 Assistant slot shell (UI rethink Phase F),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-rethink-phase-f-memory-models-assistant`'s landing commits (`git log -- spec/v1/ui-rethink-phase-f-memory-models-assistant`)
**When** the recorded verification for `ui-rethink-phase-f-memory-models-assistant` is replayed (tests, reports under `spec/v1/ui-rethink-phase-f-memory-models-assistant/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the Memory + Models screens + the Phase-6 Assistant slot shell (UI rethink Phase F)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.14: live-cockpit-unified

As the operator of the Honest Advisor,
I want the unified single-process cockpit binary,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `live-cockpit-unified`'s landing commits (`git log -- spec/v1/live-cockpit-unified`)
**When** the recorded verification for `live-cockpit-unified` is replayed (tests, reports under `spec/v1/live-cockpit-unified/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the unified single-process cockpit binary
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.15: cockpit-live-dashboard-wiring

As the operator of the Honest Advisor,
I want the Live equity curve + KPI strip fed from the live agent,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: the Live equity curve + KPI strip fed from the live agent
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.16: live-equity-history-durable

As the operator of the Honest Advisor,
I want durable equity history surviving cockpit_live restarts,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `live-equity-history-durable`'s landing commits (`git log -- spec/v1/live-equity-history-durable`)
**When** the recorded verification for `live-equity-history-durable` is replayed (tests, reports under `spec/v1/live-equity-history-durable/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: durable equity history surviving cockpit_live restarts
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.17: paper-mode-equity-wiring

As the operator of the Honest Advisor,
I want real (not flat-line) paper-mode cockpit equity,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `paper-mode-equity-wiring`'s landing commits (`git log -- spec/v1/paper-mode-equity-wiring`)
**When** the recorded verification for `paper-mode-equity-wiring` is replayed (tests, reports under `spec/v1/paper-mode-equity-wiring/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: real (not flat-line) paper-mode cockpit equity
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.18: cockpit-baseline-panel

As the operator of the Honest Advisor,
I want the Baseline panel surfacing the shipped passive buy-and-hold result,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: the Baseline panel surfacing the shipped passive buy-and-hold result
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.19: cockpit-reports-viewer

As the operator of the Honest Advisor,
I want the in-cockpit Reports screen browsing the committed backtest-report corpus with KPI strip + markdown body + equity curve,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `cockpit-reports-viewer`'s landing commits (`git log -- spec/v1/cockpit-reports-viewer`)
**When** the recorded verification for `cockpit-reports-viewer` is replayed (tests, reports under `spec/v1/cockpit-reports-viewer/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the in-cockpit Reports screen browsing the committed backtest-report corpus with KPI strip + markdown body + equity curve
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.20: backtest-equity-companion

As the operator of the Honest Advisor,
I want companion per-bar equity CSVs emitted by backtest runs so Reports + viewer render a populated curve + drawdown band (with the stem-match fix and curve-only picker default),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `backtest-equity-companion`'s landing commits (`git log -- spec/v1/backtest-equity-companion`)
**When** the recorded verification for `backtest-equity-companion` is replayed (tests, reports under `spec/v1/backtest-equity-companion/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: companion per-bar equity CSVs emitted by backtest runs so Reports + viewer render a populated curve + drawdown band (with the stem-match fix and curve-only picker default)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.21: cockpit-activity-status-bar

As the operator of the Honest Advisor,
I want the continuously-updated "what is the cockpit doing right now" activity bar with audit-ledger and LLM-call producers,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `cockpit-activity-status-bar`'s landing commits (`git log -- spec/v1/cockpit-activity-status-bar`)
**When** the recorded verification for `cockpit-activity-status-bar` is replayed (tests, reports under `spec/v1/cockpit-activity-status-bar/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the continuously-updated "what is the cockpit doing right now" activity bar with audit-ledger and LLM-call producers
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.22: cockpit-toast-queue

As the operator of the Honest Advisor,
I want the bounded toast queue replacing the single-slot REPLACE semantic,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `cockpit-toast-queue`'s landing commits (`git log -- spec/v1/cockpit-toast-queue`)
**When** the recorded verification for `cockpit-toast-queue` is replayed (tests, reports under `spec/v1/cockpit-toast-queue/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the bounded toast queue replacing the single-slot REPLACE semantic
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.23: cockpit-toast-queue-v0.2.0-cleanup

As the operator of the Honest Advisor,
I want the v0.2.0 toast cleanup retiring the legacy toast_message field,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `cockpit-toast-queue-v0.2.0-cleanup`'s landing commits (`git log -- spec/v1/cockpit-toast-queue-v0.2.0-cleanup`)
**When** the recorded verification for `cockpit-toast-queue-v0.2.0-cleanup` is replayed (tests, reports under `spec/v1/cockpit-toast-queue-v0.2.0-cleanup/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the v0.2.0 toast cleanup retiring the legacy toast_message field
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.24: cockpit-training-control

As the operator of the Honest Advisor,
I want the operator-driven train_tcn launcher,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `cockpit-training-control`'s landing commits (`git log -- spec/v1/cockpit-training-control`)
**When** the recorded verification for `cockpit-training-control` is replayed (tests, reports under `spec/v1/cockpit-training-control/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the operator-driven train_tcn launcher
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.25: cockpit-training-pressed-wiring

As the operator of the Honest Advisor,
I want the Train-button pressed wiring,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `cockpit-training-pressed-wiring`'s landing commits (`git log -- spec/v1/cockpit-training-pressed-wiring`)
**When** the recorded verification for `cockpit-training-pressed-wiring` is replayed (tests, reports under `spec/v1/cockpit-training-pressed-wiring/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the Train-button pressed wiring
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.26: cockpit-performance-and-input-responsiveness

As the operator of the Honest Advisor,
I want the input-responsiveness + render-performance pass,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `cockpit-performance-and-input-responsiveness`'s landing commits (`git log -- spec/v1/cockpit-performance-and-input-responsiveness`)
**When** the recorded verification for `cockpit-performance-and-input-responsiveness` is replayed (tests, reports under `spec/v1/cockpit-performance-and-input-responsiveness/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the input-responsiveness + render-performance pass
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.27: cockpit-render-regression

As the operator of the Honest Advisor,
I want the render-regression diagnosis + quality-gate overhaul,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `cockpit-render-regression`'s landing commits (`git log -- spec/v1/cockpit-render-regression`)
**When** the recorded verification for `cockpit-render-regression` is replayed (tests, reports under `spec/v1/cockpit-render-regression/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the render-regression diagnosis + quality-gate overhaul
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.28: cockpit-chart-cache

As the operator of the Honest Advisor,
I want the chart canvas::Cache hover-smoothness measure - Phase 1 MEASURE returned NO-GO (deprecated by measurement),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `cockpit-chart-cache`'s landing commits (`git log -- spec/v1/cockpit-chart-cache`)
**When** the recorded verification for `cockpit-chart-cache` is replayed (tests, reports under `spec/v1/cockpit-chart-cache/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the chart canvas::Cache hover-smoothness measure - Phase 1 MEASURE returned NO-GO (deprecated by measurement)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.29: chart-canvas-overhaul

As the operator of the Honest Advisor,
I want the chart canvas rewrite,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `chart-canvas-overhaul`'s landing commits (`git log -- spec/v1/chart-canvas-overhaul`)
**When** the recorded verification for `chart-canvas-overhaul` is replayed (tests, reports under `spec/v1/chart-canvas-overhaul/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the chart canvas rewrite
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.30: chart-buy-sell-emphasis

As the operator of the Honest Advisor,
I want buy/sell marker emphasis fed from the audit ledger,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `chart-buy-sell-emphasis`'s landing commits (`git log -- spec/v1/chart-buy-sell-emphasis`)
**When** the recorded verification for `chart-buy-sell-emphasis` is replayed (tests, reports under `spec/v1/chart-buy-sell-emphasis/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: buy/sell marker emphasis fed from the audit ledger
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.31: chart-x-axis-local-time

As the operator of the Honest Advisor,
I want the local-time chart x-axis (atomic test-mode override; fixed a set_var data race),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `chart-x-axis-local-time`'s landing commits (`git log -- spec/v1/chart-x-axis-local-time`)
**When** the recorded verification for `chart-x-axis-local-time` is replayed (tests, reports under `spec/v1/chart-x-axis-local-time/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the local-time chart x-axis (atomic test-mode override; fixed a set_var data race)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.32: chart-fixture-line-clipping

As the operator of the Honest Advisor,
I want the canvas-clip fix via the vendored iced_tiny_skia fork (operator-locked maintenance contract),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `chart-fixture-line-clipping`'s landing commits (`git log -- spec/v1/chart-fixture-line-clipping`)
**When** the recorded verification for `chart-fixture-line-clipping` is replayed (tests, reports under `spec/v1/chart-fixture-line-clipping/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the canvas-clip fix via the vendored iced_tiny_skia fork (operator-locked maintenance contract)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.33: tape-row-audit-modal

As the operator of the Honest Advisor,
I want click-a-tape-row opening its audit-transaction modal,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `tape-row-audit-modal`'s landing commits (`git log -- spec/v1/tape-row-audit-modal`)
**When** the recorded verification for `tape-row-audit-modal` is replayed (tests, reports under `spec/v1/tape-row-audit-modal/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: click-a-tape-row opening its audit-transaction modal
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.34: journal-transactions-metadata

As the operator of the Honest Advisor,
I want the journal-transactions metadata reader,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `journal-transactions-metadata`'s landing commits (`git log -- spec/v1/journal-transactions-metadata`)
**When** the recorded verification for `journal-transactions-metadata` is replayed (tests, reports under `spec/v1/journal-transactions-metadata/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the journal-transactions metadata reader
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.35: real-mtm-unrealized-pnl

As the operator of the Honest Advisor,
I want real mark-to-market unrealized P&L,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `real-mtm-unrealized-pnl`'s landing commits (`git log -- spec/v1/real-mtm-unrealized-pnl`)
**When** the recorded verification for `real-mtm-unrealized-pnl` is replayed (tests, reports under `spec/v1/real-mtm-unrealized-pnl/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: real mark-to-market unrealized P&L
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.36: per-symbol-position-accounts

As the operator of the Honest Advisor,
I want per-symbol position accounts in the ledger,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `per-symbol-position-accounts`'s landing commits (`git log -- spec/v1/per-symbol-position-accounts`)
**When** the recorded verification for `per-symbol-position-accounts` is replayed (tests, reports under `spec/v1/per-symbol-position-accounts/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: per-symbol position accounts in the ledger
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.37: lab-run-save-compare

As the operator of the Honest Advisor,
I want real-data strategy checking in the Lab with durable reports,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lab-run-save-compare`'s landing commits (`git log -- spec/v1/lab-run-save-compare`)
**When** the recorded verification for `lab-run-save-compare` is replayed (tests, reports under `spec/v1/lab-run-save-compare/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: real-data strategy checking in the Lab with durable reports
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.38: lab-compare-equity-overlay

As the operator of the Honest Advisor,
I want the two-run equity overlay on the Compare screen,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lab-compare-equity-overlay`'s landing commits (`git log -- spec/v1/lab-compare-equity-overlay`)
**When** the recorded verification for `lab-compare-equity-overlay` is replayed (tests, reports under `spec/v1/lab-compare-equity-overlay/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the two-run equity overlay on the Compare screen
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.39: lab-end-to-end-v2

As the operator of the Honest Advisor,
I want the Lab end-to-end v2 pass closing the Phase A/B gaps and adding a progress bar,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lab-end-to-end-v2`'s landing commits (`git log -- spec/v1/lab-end-to-end-v2`)
**When** the recorded verification for `lab-end-to-end-v2` is replayed (tests, reports under `spec/v1/lab-end-to-end-v2/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the Lab end-to-end v2 pass closing the Phase A/B gaps and adding a progress bar
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.40: lab-polish-round-2

As the operator of the Honest Advisor,
I want Lab polish round 2: position curve + param tuning + UI density,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lab-polish-round-2`'s landing commits (`git log -- spec/v1/lab-polish-round-2`)
**When** the recorded verification for `lab-polish-round-2` is replayed (tests, reports under `spec/v1/lab-polish-round-2/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: Lab polish round 2: position curve + param tuning + UI density
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.41: lab-recipe-test-harness

As the operator of the Honest Advisor,
I want the Lab recipe / subscription test harness,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lab-recipe-test-harness`'s landing commits (`git log -- spec/v1/lab-recipe-test-harness`)
**When** the recorded verification for `lab-recipe-test-harness` is replayed (tests, reports under `spec/v1/lab-recipe-test-harness/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the Lab recipe / subscription test harness
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.42: lab-recipe-test-harness-v0.2.0-cross-surface-extension

As the operator of the Honest Advisor,
I want the v0.2.0 cross-surface extension of the recipe harness,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lab-recipe-test-harness-v0.2.0-cross-surface-extension`'s landing commits (`git log -- spec/v1/lab-recipe-test-harness-v0.2.0-cross-surface-extension`)
**When** the recorded verification for `lab-recipe-test-harness-v0.2.0-cross-surface-extension` is replayed (tests, reports under `spec/v1/lab-recipe-test-harness-v0.2.0-cross-surface-extension/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the v0.2.0 cross-surface extension of the recipe harness
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.43: lab-yahoo-realdata

As the operator of the Honest Advisor,
I want the multi-asset Yahoo data pivot for the Lab,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lab-yahoo-realdata`'s landing commits (`git log -- spec/v1/lab-yahoo-realdata`)
**When** the recorded verification for `lab-yahoo-realdata` is replayed (tests, reports under `spec/v1/lab-yahoo-realdata/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the multi-asset Yahoo data pivot for the Lab
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.44: lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge

As the operator of the Honest Advisor,
I want the v0.1.2 ETH-USD anchor + cache-state summary badge,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge`'s landing commits (`git log -- spec/v1/lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge`)
**When** the recorded verification for `lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge` is replayed (tests, reports under `spec/v1/lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the v0.1.2 ETH-USD anchor + cache-state summary badge
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.45: lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1

As the operator of the Honest Advisor,
I want the v0.1.3 REVISION front-matter + Binance ETH H1 corpus,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1`'s landing commits (`git log -- spec/v1/lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1`)
**When** the recorded verification for `lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1` is replayed (tests, reports under `spec/v1/lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the v0.1.3 REVISION front-matter + Binance ETH H1 corpus
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.46: lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit

As the operator of the Honest Advisor,
I want the v0.1.4 bulk-ticker re-emit (9 new tickers + ETH-daily redo) - retired,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit`'s landing commits (`git log -- spec/v1/lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit`)
**When** the recorded verification for `lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit` is replayed (tests, reports under `spec/v1/lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the v0.1.4 bulk-ticker re-emit (9 new tickers + ETH-daily redo) - retired
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.47: lab-yahoo-empty-range-ux

As the operator of the Honest Advisor,
I want the empty-date-range UX handling in the Lab,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `lab-yahoo-empty-range-ux`'s landing commits (`git log -- spec/v1/lab-yahoo-empty-range-ux`)
**When** the recorded verification for `lab-yahoo-empty-range-ux` is replayed (tests, reports under `spec/v1/lab-yahoo-empty-range-ux/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the empty-date-range UX handling in the Lab
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.48: bug-64-d11-attempt-3-yahoo-run-runtime-context

As the operator of the Honest Advisor,
I want the Yahoo+Run runtime-context + cancellation fix (bug 64 / D11 attempt 3),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `bug-64-d11-attempt-3-yahoo-run-runtime-context`'s landing commits (`git log -- spec/v1/bug-64-d11-attempt-3-yahoo-run-runtime-context`)
**When** the recorded verification for `bug-64-d11-attempt-3-yahoo-run-runtime-context` is replayed (tests, reports under `spec/v1/bug-64-d11-attempt-3-yahoo-run-runtime-context/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the Yahoo+Run runtime-context + cancellation fix (bug 64 / D11 attempt 3)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.49: iced-native-widgets

As the operator of the Honest Advisor,
I want the migration to native iced widgets,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `iced-native-widgets`'s landing commits (`git log -- spec/v1/iced-native-widgets`)
**When** the recorded verification for `iced-native-widgets` is replayed (tests, reports under `spec/v1/iced-native-widgets/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the migration to native iced widgets
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.50: iced-aw-cherry-pick

As the operator of the Honest Advisor,
I want the iced_aw cherry-pick bridge used during the native-widget migration,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `iced-aw-cherry-pick`'s landing commits (`git log -- spec/v1/iced-aw-cherry-pick`)
**When** the recorded verification for `iced-aw-cherry-pick` is replayed (tests, reports under `spec/v1/iced-aw-cherry-pick/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the iced_aw cherry-pick bridge used during the native-widget migration
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.51: ui-drop-iced-aw

As the operator of the Honest Advisor,
I want dropping the iced_aw / iced_fonts dependencies,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-drop-iced-aw`'s landing commits (`git log -- spec/v1/ui-drop-iced-aw`)
**When** the recorded verification for `ui-drop-iced-aw` is replayed (tests, reports under `spec/v1/ui-drop-iced-aw/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: dropping the iced_aw / iced_fonts dependencies
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.52: ui-gallery-bin

As the operator of the Honest Advisor,
I want the widget-gallery binary,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-gallery-bin`'s landing commits (`git log -- spec/v1/ui-gallery-bin`)
**When** the recorded verification for `ui-gallery-bin` is replayed (tests, reports under `spec/v1/ui-gallery-bin/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the widget-gallery binary
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.53: ui-gallery-table-cell

As the operator of the Honest Advisor,
I want the widget-gallery table-cell bounds fix (draft brief; not built),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the recorded brief in `spec/ui-gallery-table-cell/feature.md`
**When** the operator schedules the work (post do-not-build-register check)
**Then** the story delivers: the widget-gallery table-cell bounds fix (draft brief; not built)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.54: ui-contrast-asserter

As the operator of the Honest Advisor,
I want the WCAG contrast asserter (v0.2 enforcing gate, UI_CONTRAST_MODE, 6 ratified opt-outs),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-contrast-asserter`'s landing commits (`git log -- spec/v1/ui-contrast-asserter`)
**When** the recorded verification for `ui-contrast-asserter` is replayed (tests, reports under `spec/v1/ui-contrast-asserter/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the WCAG contrast asserter (v0.2 enforcing gate, UI_CONTRAST_MODE, 6 ratified opt-outs)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.55: ui-quality-gate-overhaul

As the operator of the Honest Advisor,
I want the UI quality-gate overhaul,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-quality-gate-overhaul`'s landing commits (`git log -- spec/v1/ui-quality-gate-overhaul`)
**When** the recorded verification for `ui-quality-gate-overhaul` is replayed (tests, reports under `spec/v1/ui-quality-gate-overhaul/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the UI quality-gate overhaul
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.56: ui-headless-emulator

As the operator of the Honest Advisor,
I want the headless render adapter for UI tests,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-headless-emulator`'s landing commits (`git log -- spec/v1/ui-headless-emulator`)
**When** the recorded verification for `ui-headless-emulator` is replayed (tests, reports under `spec/v1/ui-headless-emulator/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the headless render adapter for UI tests
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.57: ui-session-journal-iced-tester

As the operator of the Honest Advisor,
I want the iced_tester session-journal adapter,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-session-journal-iced-tester`'s landing commits (`git log -- spec/v1/ui-session-journal-iced-tester`)
**When** the recorded verification for `ui-session-journal-iced-tester` is replayed (tests, reports under `spec/v1/ui-session-journal-iced-tester/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the iced_tester session-journal adapter
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.58: ui-test-harness-bootstrap

As the operator of the Honest Advisor,
I want the panel-snapshot test harness bootstrap,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-test-harness-bootstrap`'s landing commits (`git log -- spec/v1/ui-test-harness-bootstrap`)
**When** the recorded verification for `ui-test-harness-bootstrap` is replayed (tests, reports under `spec/v1/ui-test-harness-bootstrap/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the panel-snapshot test harness bootstrap
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.59: ui-test-harness-viewport-matrix

As the operator of the Honest Advisor,
I want the multi-viewport snapshot matrix,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `ui-test-harness-viewport-matrix`'s landing commits (`git log -- spec/v1/ui-test-harness-viewport-matrix`)
**When** the recorded verification for `ui-test-harness-viewport-matrix` is replayed (tests, reports under `spec/v1/ui-test-harness-viewport-matrix/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the multi-viewport snapshot matrix
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.60: visual-fail-html-reporter

As the operator of the Honest Advisor,
I want the HTML viewer for visual-test failures,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the repo history at `visual-fail-html-reporter`'s landing commits (`git log -- spec/v1/visual-fail-html-reporter`)
**When** the recorded verification for `visual-fail-html-reporter` is replayed (tests, reports under `spec/v1/visual-fail-html-reporter/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the HTML viewer for visual-test failures
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.61: iced-ecosystem-evaluation

As the operator of the Honest Advisor,
I want the iced ecosystem research/scoping brief (candidate; no code changes),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the recorded brief in `spec/iced-ecosystem-evaluation/feature.md`
**When** the operator schedules the work (post do-not-build-register check)
**Then** the story delivers: the iced ecosystem research/scoping brief (candidate; no code changes)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.62: cockpit-app-bundle

As the operator of the Honest Advisor,
I want macOS .app packaging for dock + cmd-tab + Spotlight icons (candidate; not built),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the recorded brief in `spec/cockpit-app-bundle/feature.md`
**When** the operator schedules the work (post do-not-build-register check)
**Then** the story delivers: macOS .app packaging for dock + cmd-tab + Spotlight icons (candidate; not built)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 2.63: lab-recipe-test-harness-v0-3-extension

As the operator of the Honest Advisor,
I want the v0.3.0+ recipe/subscription harness extension - the one genuinely-open forward build item (robustness gate cleared, awaiting an analyst spawn),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

**Acceptance Criteria:**

**Given** the shipped v0.2.0 recipe harness and the open backlog entry (PRD §13 Q2)
**When** the operator schedules (or retires) the v0.3.0+ extension
**Then** either an analyst spawn opens the feature with its own brief, or the backlog entry is closed as retired - no silent limbo
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

## Epic 3: Advisor MVP (F1-F9 + EUR-FX + dynamic data + PIT discipline)

The 2026-06-19 pivot product: the guided coin+budget journey - bake-off, robustness-gated ranking, forward plan, forward paper-trade - plus the pre-registered arm-class expansions and the point-in-time data primitive.

### Story 3.1: advisor-bakeoff-ranking

As the operator of the Honest Advisor,
I want the strategy bake-off + ranking engine (run_bakeoff -> BakeoffReport; Fragile-ineligible -> Sharpe -> return -> drawdown -> id; buy-and-hold always the benchmark arm; structured Recommendation) plus the Leaderboard/guided-input surfaces (F1+F2, with F3 and the leaderboard-inspect iterations landing on the same folder),
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `advisor-bakeoff-ranking`'s landing commits (`git log -- spec/v1/advisor-bakeoff-ranking`)
**When** the recorded verification for `advisor-bakeoff-ranking` is replayed (tests, reports under `spec/v1/advisor-bakeoff-ranking/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the strategy bake-off + ranking engine (run_bakeoff -> BakeoffReport; Fragile-ineligible -> Sharpe -> return -> drawdown -> id; buy-and-hold always the benchmark arm; structured Recommendation) plus the Leaderboard/guided-input surfaces (F1+F2, with F3 and the leaderboard-inspect iterations landing on the same folder)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.2: advisor-forward-paper

As the operator of the Honest Advisor,
I want the forward paper-trade of the SELECTED strategy at the budget cap (F4 budget-aware sizing with day-1 divergence e2e; F5 paper_loop_supervisor hot-swap; F5b real-strategy fidelity),
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `advisor-forward-paper`'s landing commits (`git log -- spec/v1/advisor-forward-paper`)
**When** the recorded verification for `advisor-forward-paper` is replayed (tests, reports under `spec/v1/advisor-forward-paper/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the forward paper-trade of the SELECTED strategy at the budget cap (F4 budget-aware sizing with day-1 divergence e2e; F5 paper_loop_supervisor hot-swap; F5b real-strategy fidelity)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.3: advisor-forward-plan

As the operator of the Honest Advisor,
I want the honest, conditional forward buy/sell plan (F6): IF/THEN rules faithful to the real TOMLs, not-a-prediction framing, configurable 1-30d horizon,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `advisor-forward-plan`'s landing commits (`git log -- spec/v1/advisor-forward-plan`)
**When** the recorded verification for `advisor-forward-plan` is replayed (tests, reports under `spec/v1/advisor-forward-plan/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the honest, conditional forward buy/sell plan (F6): IF/THEN rules faithful to the real TOMLs, not-a-prediction framing, configurable 1-30d horizon
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.4: advisor-dynamic-data

As the operator of the Honest Advisor,
I want on-demand Binance fetch for any coin + window outside the pinned corpus (git-ignored cache, anchor-safe by construction),
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `advisor-dynamic-data`'s landing commits (`git log -- spec/v1/advisor-dynamic-data`)
**When** the recorded verification for `advisor-dynamic-data` is replayed (tests, reports under `spec/v1/advisor-dynamic-data/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: on-demand Binance fetch for any coin + window outside the pinned corpus (git-ignored cache, anchor-safe by construction)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.5: advisor-ensemble

As the operator of the Honest Advisor,
I want strategy-mix vote ensembles (F8) + activation of the robustness gate in the live bake-off (a fragile candidate is shown but never crowned),
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `advisor-ensemble`'s landing commits (`git log -- spec/v1/advisor-ensemble`)
**When** the recorded verification for `advisor-ensemble` is replayed (tests, reports under `spec/v1/advisor-ensemble/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: strategy-mix vote ensembles (F8) + activation of the robustness gate in the live bake-off (a fragile candidate is shown but never crowned)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.6: advisor-llm-narration

As the operator of the Honest Advisor,
I want opt-in faithful LLM narration of the crowned pick (F9) guarded by the deterministic check_faithful post-check with templated-copy fallback, plus the F6+F9 live last-mile display recipes,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `advisor-llm-narration`'s landing commits (`git log -- spec/v1/advisor-llm-narration`)
**When** the recorded verification for `advisor-llm-narration` is replayed (tests, reports under `spec/v1/advisor-llm-narration/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: opt-in faithful LLM narration of the crowned pick (F9) guarded by the deterministic check_faithful post-check with templated-copy fallback, plus the F6+F9 live last-mile display recipes
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.7: advisor-eur-fx

As the operator of the Honest Advisor,
I want honest EUR->USDT budget conversion (F7): one-time conversion at a configurable static rate with the "EUR 200 ~ $216.00 (at 1.08 EUR/USD, config)" display and a day-1 conversion-applied gate,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `advisor-eur-fx`'s landing commits (`git log -- spec/v1/advisor-eur-fx`)
**When** the recorded verification for `advisor-eur-fx` is replayed (tests, reports under `spec/v1/advisor-eur-fx/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: honest EUR->USDT budget conversion (F7): one-time conversion at a configurable static rate with the "EUR 200 ~ $216.00 (at 1.08 EUR/USD, config)" display and a day-1 conversion-applied gate
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.8: advisor-benchmark-robustness

As the operator of the Honest Advisor,
I want the benchmark-exemption robustness-honesty fix (B1): buy-and-hold exempt from AllFragile and always crown-eligible, restoring BenchmarkWins as the modal honest outcome,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `advisor-benchmark-robustness`'s landing commits (`git log -- spec/v1/advisor-benchmark-robustness`)
**When** the recorded verification for `advisor-benchmark-robustness` is replayed (tests, reports under `spec/v1/advisor-benchmark-robustness/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the benchmark-exemption robustness-honesty fix (B1): buy-and-hold exempt from AllFragile and always crown-eligible, restoring BenchmarkWins as the modal honest outcome
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.9: advisor-combination-search

As the operator of the Honest Advisor,
I want the pre-registered 6-arm combination slate (13-arm field) scored through the identical frozen gate - returning the honest null (no combination cleared the gate),
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `advisor-combination-search`'s landing commits (`git log -- spec/v1/advisor-combination-search`)
**When** the recorded verification for `advisor-combination-search` is replayed (tests, reports under `spec/v1/advisor-combination-search/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the pre-registered 6-arm combination slate (13-arm field) scored through the identical frozen gate - returning the honest null (no combination cleared the gate)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.10: advisor-short-selling

As the operator of the Honest Advisor,
I want simulated directional shorts (paper-only, separate 5-arm slate): ported margin/liquidation/funding engine, honest negative P&L + unbounded-loss disclaimer - short timing fails like long timing,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `advisor-short-selling`'s landing commits (`git log -- spec/v1/advisor-short-selling`)
**When** the recorded verification for `advisor-short-selling` is replayed (tests, reports under `spec/v1/advisor-short-selling/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: simulated directional shorts (paper-only, separate 5-arm slate): ported margin/liquidation/funding engine, honest negative P&L + unbounded-loss disclaimer - short timing fails like long timing
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.11: leaderboard-timeframe-capital

As the operator of the Honest Advisor,
I want the bake-off tune knobs: H1/H4/D1 timeframe resampling (may change ranking) and starting-capital control (does not - and the UI says so),
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `leaderboard-timeframe-capital`'s landing commits (`git log -- spec/v1/leaderboard-timeframe-capital`)
**When** the recorded verification for `leaderboard-timeframe-capital` is replayed (tests, reports under `spec/v1/leaderboard-timeframe-capital/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the bake-off tune knobs: H1/H4/D1 timeframe resampling (may change ranking) and starting-capital control (does not - and the UI says so)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.12: advisor-param-tuning

As the operator of the Honest Advisor,
I want the gate-tied hyperparameter sweep editor (Tune screen): every config scored through the identical frozen gate, FRAGILE configs promote-locked,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `advisor-param-tuning`'s landing commits (`git log -- spec/v1/advisor-param-tuning`)
**When** the recorded verification for `advisor-param-tuning` is replayed (tests, reports under `spec/v1/advisor-param-tuning/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the gate-tied hyperparameter sweep editor (Tune screen): every config scored through the identical frozen gate, FRAGILE configs promote-locked
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.13: advisor-param-promotion

As the operator of the Honest Advisor,
I want promotion of a surviving (non-FRAGILE) tuned config into the forward plan + paper-run, with the tuned-rules honesty header,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `advisor-param-promotion`'s landing commits (`git log -- spec/v1/advisor-param-promotion`)
**When** the recorded verification for `advisor-param-promotion` is replayed (tests, reports under `spec/v1/advisor-param-promotion/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: promotion of a surviving (non-FRAGILE) tuned config into the forward plan + paper-run, with the tuned-rules honesty header
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.14: advisor-signal-library-expansion

As the operator of the Honest Advisor,
I want the pre-registered 5-arm signal-library slate (Donchian break/floor, volume breakout, ROC momentum, OBV - new DSL primitive) - all FRAGILE, the expected null,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `advisor-signal-library-expansion`'s landing commits (`git log -- spec/v1/advisor-signal-library-expansion`)
**When** the recorded verification for `advisor-signal-library-expansion` is replayed (tests, reports under `spec/v1/advisor-signal-library-expansion/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the pre-registered 5-arm signal-library slate (Donchian break/floor, volume breakout, ROC momentum, OBV - new DSL primitive) - all FRAGILE, the expected null
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.15: advisor-options-impliedvol-probe

As the operator of the Honest Advisor,
I want the Deribit DVOL implied-vol regime probe (v0.dvol_regime, locked W=30, PIT-joined) - FRAGILE on BTC+ETH, the pre-registered null,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `tester-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: the Deribit DVOL implied-vol regime probe (v0.dvol_regime, locked W=30, PIT-joined) - FRAGILE on BTC+ETH, the pre-registered null
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.16: advisor-crossasset-macro-regime

As the operator of the Honest Advisor,
I want the macro risk-on/off probe (v0.macro_riskon over ^GSPC/DXY/^TNX) + the durable market-calendar layer - FRAGILE, the pre-registered null,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the built-and-verified state frozen at frontmatter `tester-done` (2026-06-17 spec compression)
**When** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived
**Then** the delivered behaviour stands as recorded: the macro risk-on/off probe (v0.macro_riskon over ^GSPC/DXY/^TNX) + the durable market-calendar layer - FRAGILE, the pre-registered null
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.17: point-in-time-data-discipline

As the operator of the Honest Advisor,
I want the core::pit PitSeries/AsOf primitive making look-ahead unrepresentable at the type level (trybuild compile-fail proof), consolidating the hand-rolled as-of joins,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the repo history at `point-in-time-data-discipline`'s landing commits (`git log -- spec/v1/point-in-time-data-discipline`)
**When** the recorded verification for `point-in-time-data-discipline` is replayed (tests, reports under `spec/v1/point-in-time-data-discipline/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the core::pit PitSeries/AsOf primitive making look-ahead unrepresentable at the type level (trybuild compile-fail proof), consolidating the hand-rolled as-of joins
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 3.18: advisor-reflection-decision-loop

As the operator of the Honest Advisor,
I want the C4 reflection decision-support memory surface for the advisor (the honest C4) - architecture done, build pending an operator decision,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

**Acceptance Criteria:**

**Given** the completed C4 architecture in spec/advisor-reflection-decision-loop/
**When** the operator green-lights the build (or parks it via the do-not-build check)
**Then** the story moves to dev with the arch-done design as its context - until then it stays ready-for-dev, honestly not built
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

## Epic 4: v2 Research-Driven Credibility Tranche

The 11-feature tranche distilled from the 900-paper research program: overfitting scorecard, churn/tail transparency, confidence-not-verdict framing, de-risk overlays, cost realism, narration faithfulness, the no-alpha CI capstone.

### Story 4.1: advisor-overfitting-scorecard

As the operator of the Honest Advisor,
I want the report-only overfitting scorecard (N_eff -> DSR -> MinBTL) beside every recommendation - additive to the FROZEN gate, never a veto (P0-1),
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

**Acceptance Criteria:**

**Given** the repo history at `advisor-overfitting-scorecard`'s landing commits (`git log -- spec/v2/advisor-overfitting-scorecard`)
**When** the recorded verification for `advisor-overfitting-scorecard` is replayed (tests, reports under `spec/v2/advisor-overfitting-scorecard/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the report-only overfitting scorecard (N_eff -> DSR -> MinBTL) beside every recommendation - additive to the FROZEN gate, never a veto (P0-1)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 4.2: advisor-turnover-and-tail-metrics

As the operator of the Honest Advisor,
I want the Churn (turnover) column + the coherent "Risk story" tail block (CVaR / median / skew) on the leaderboard (P1-1 + P1-2),
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

**Acceptance Criteria:**

**Given** the repo history at `advisor-turnover-and-tail-metrics`'s landing commits (`git log -- spec/v2/advisor-turnover-and-tail-metrics`)
**When** the recorded verification for `advisor-turnover-and-tail-metrics` is replayed (tests, reports under `spec/v2/advisor-turnover-and-tail-metrics/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the Churn (turnover) column + the coherent "Risk story" tail block (CVaR / median / skew) on the leaderboard (P1-1 + P1-2)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 4.3: advisor-confidence-not-verdict

As the operator of the Honest Advisor,
I want the confidence-check-not-verdict reframing of the recommendation surface (P0-3),
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

**Acceptance Criteria:**

**Given** the repo history at `advisor-confidence-not-verdict`'s landing commits (`git log -- spec/v2/advisor-confidence-not-verdict`)
**When** the recorded verification for `advisor-confidence-not-verdict` is replayed (tests, reports under `spec/v2/advisor-confidence-not-verdict/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the confidence-check-not-verdict reframing of the recommendation surface (P0-3)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 4.4: advisor-forward-fidelity-coverage

As the operator of the Honest Advisor,
I want forward-run coverage for all 14 post-F5b arms so crowning any arm cannot bail the forward paper-run (R1),
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

**Acceptance Criteria:**

**Given** the repo history at `advisor-forward-fidelity-coverage`'s landing commits (`git log -- spec/v2/advisor-forward-fidelity-coverage`)
**When** the recorded verification for `advisor-forward-fidelity-coverage` is replayed (tests, reports under `spec/v2/advisor-forward-fidelity-coverage/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: forward-run coverage for all 14 post-F5b arms so crowning any arm cannot bail the forward paper-run (R1)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 4.5: advisor-vol-estimator

As the operator of the Honest Advisor,
I want the shared multi-horizon sigma-hat volatility estimator feeding the de-risk overlays (P1-5); carries the phase-2c test-report umbrella,
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

**Acceptance Criteria:**

**Given** the repo history at `advisor-vol-estimator`'s landing commits (`git log -- spec/v2/advisor-vol-estimator`)
**When** the recorded verification for `advisor-vol-estimator` is replayed (tests, reports under `spec/v2/advisor-vol-estimator/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the shared multi-horizon sigma-hat volatility estimator feeding the de-risk overlays (P1-5); carries the phase-2c test-report umbrella
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 4.6: advisor-vol-overlay-reposition

As the operator of the Honest Advisor,
I want the vol-targeting overlay repositioned as an honest de-risk-only sizing choice on the crowned pick, with the day-1 divergence e2e (P1-4),
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

**Acceptance Criteria:**

**Given** the repo history at `advisor-vol-overlay-reposition`'s landing commits (`git log -- spec/v2/advisor-vol-overlay-reposition`)
**When** the recorded verification for `advisor-vol-overlay-reposition` is replayed (tests, reports under `spec/v2/advisor-vol-overlay-reposition/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the vol-targeting overlay repositioned as an honest de-risk-only sizing choice on the crowned pick, with the day-1 divergence e2e (P1-4)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 4.7: advisor-drawdown-control-overlay

As the operator of the Honest Advisor,
I want the drawdown-control sizing overlay (high-water-mark restart, CPPI-style 20% floor) with day-1 divergence e2e (P1-3),
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

**Acceptance Criteria:**

**Given** the repo history at `advisor-drawdown-control-overlay`'s landing commits (`git log -- spec/v2/advisor-drawdown-control-overlay`)
**When** the recorded verification for `advisor-drawdown-control-overlay` is replayed (tests, reports under `spec/v2/advisor-drawdown-control-overlay/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the drawdown-control sizing overlay (high-water-mark restart, CPPI-style 20% floor) with day-1 divergence e2e (P1-3)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 4.8: advisor-cost-model-opt-in

As the operator of the Honest Advisor,
I want the opt-in VolScaledSpread cost variant + venue-trust map + fee-sensitivity read (default LinearBps stays anchor-stable) (P1-6),
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

**Acceptance Criteria:**

**Given** the repo history at `advisor-cost-model-opt-in`'s landing commits (`git log -- spec/v2/advisor-cost-model-opt-in`)
**When** the recorded verification for `advisor-cost-model-opt-in` is replayed (tests, reports under `spec/v2/advisor-cost-model-opt-in/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the opt-in VolScaledSpread cost variant + venue-trust map + fee-sensitivity read (default LinearBps stays anchor-stable) (P1-6)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 4.9: advisor-narration-faithfulness

As the operator of the Honest Advisor,
I want hardened F9 narration faithfulness: verbatim-number matching + the expanded predict/advise banned-phrase list (P2-1),
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

**Acceptance Criteria:**

**Given** the repo history at `advisor-narration-faithfulness`'s landing commits (`git log -- spec/v2/advisor-narration-faithfulness`)
**When** the recorded verification for `advisor-narration-faithfulness` is replayed (tests, reports under `spec/v2/advisor-narration-faithfulness/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: hardened F9 narration faithfulness: verbatim-number matching + the expanded predict/advise banned-phrase list (P2-1)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 4.10: advisor-no-alpha-gate-ci

As the operator of the Honest Advisor,
I want the null-falsification CI (GBM/GARCH/OU pure noise): the frozen gate alone crowns noise ~1 in 5 seeds and the DSR scorecard caught every chance-crown (P2-2),
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

**Acceptance Criteria:**

**Given** the repo history at `advisor-no-alpha-gate-ci`'s landing commits (`git log -- spec/v2/advisor-no-alpha-gate-ci`)
**When** the recorded verification for `advisor-no-alpha-gate-ci` is replayed (tests, reports under `spec/v2/advisor-no-alpha-gate-ci/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the null-falsification CI (GBM/GARCH/OU pure noise): the frozen gate alone crowns noise ~1 in 5 seeds and the DSR scorecard caught every chance-crown (P2-2)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 4.11: phase-2d

As the operator of the Honest Advisor,
I want the Phase 2D test-report umbrella folder carrying the no-alpha CI run evidence (companion to advisor-no-alpha-gate-ci; not a standalone product feature),
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

**Acceptance Criteria:**

**Given** the repo history at `phase-2d`'s landing commits (`git log -- spec/v2/phase-2d`)
**When** the recorded verification for `phase-2d` is replayed (tests, reports under `spec/v2/phase-2d/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the Phase 2D test-report umbrella folder carrying the no-alpha CI run evidence (companion to advisor-no-alpha-gate-ci; not a standalone product feature)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 4.12: advisor-data-quality-surface

As the operator of the Honest Advisor,
I want the display-only DATA-stage trust/quality panel (venue provenance, trust class, survival-bias caveat, plain-language warnings) - never feeds any gate (P1-7),
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

**Acceptance Criteria:**

**Given** the repo history at `advisor-data-quality-surface`'s landing commits (`git log -- spec/v2/advisor-data-quality-surface`)
**When** the recorded verification for `advisor-data-quality-surface` is replayed (tests, reports under `spec/v2/advisor-data-quality-surface/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the display-only DATA-stage trust/quality panel (venue provenance, trust class, survival-bias caveat, plain-language warnings) - never feeds any gate (P1-7)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

## Epic 5: v3 "Prove It's Done" Close-Out

The bounded ship-readiness pass: Calibrate stage + journey stepper, corpus expansion + the P2 verdict re-run (the era-qualification evidence), and the P1/P3/P4/P5 remediation deliverables (crown-credibility band, PIT lint, lot realism, plan export).

### Story 5.1: advisor-calibrate-stage

As the operator of the Honest Advisor,
I want the first-class Calibrate stage + the DATA -> CALIBRATE -> ANALYZE -> SUGGEST stepper band (orientation affordance, render-pixel-verified) (R3-3a),
so that the shipped product is provably done, with its thesis boundary honestly mapped.

**Acceptance Criteria:**

**Given** the repo history at `advisor-calibrate-stage`'s landing commits (`git log -- spec/v3/advisor-calibrate-stage`)
**When** the recorded verification for `advisor-calibrate-stage` is replayed (tests, reports under `spec/v3/advisor-calibrate-stage/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the first-class Calibrate stage + the DATA -> CALIBRATE -> ANALYZE -> SUGGEST stepper band (orientation affordance, render-pixel-verified) (R3-3a)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 5.2: advisor-corpus-expansion

As the operator of the Honest Advisor,
I want the P2 corpus expansion (4 new pinned corpora + 2nd venue) + the ship-passive verdict re-run that era-qualified the thesis (efficiency migration; scorecard errata honored),
so that the shipped product is provably done, with its thesis boundary honestly mapped.

**Acceptance Criteria:**

**Given** the repo history at `advisor-corpus-expansion`'s landing commits (`git log -- spec/v3/advisor-corpus-expansion`)
**When** the recorded verification for `advisor-corpus-expansion` is replayed (tests, reports under `spec/v3/advisor-corpus-expansion/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the P2 corpus expansion (4 new pinned corpora + 2nd venue) + the ship-passive verdict re-run that era-qualified the thesis (efficiency migration; scorecard errata honored)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 5.3: advisor-crown-credibility

As the operator of the Honest Advisor,
I want the crown-credibility band: a DSR-failing crowned pick carries an unmissable in-body weak-evidence band (P1),
so that the shipped product is provably done, with its thesis boundary honestly mapped.

**Acceptance Criteria:**

**Given** the repo history at `advisor-crown-credibility`'s landing commits (`git log -- spec/v3/advisor-crown-credibility`)
**When** the recorded verification for `advisor-crown-credibility` is replayed (tests, reports under `spec/v3/advisor-crown-credibility/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the crown-credibility band: a DSR-failing crowned pick carries an unmissable in-body weak-evidence band (P1)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 5.4: advisor-pit-discipline

As the operator of the Honest Advisor,
I want the look-ahead lint (check_no_raw_asof_join.sh) + explicit publication_lag_ms on PitSeries; DVOL/macro joins proven as-of-correct (P3),
so that the shipped product is provably done, with its thesis boundary honestly mapped.

**Acceptance Criteria:**

**Given** the repo history at `advisor-pit-discipline`'s landing commits (`git log -- spec/v3/advisor-pit-discipline`)
**When** the recorded verification for `advisor-pit-discipline` is replayed (tests, reports under `spec/v3/advisor-pit-discipline/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the look-ahead lint (check_no_raw_asof_join.sh) + explicit publication_lag_ms on PitSeries; DVOL/macro joins proven as-of-correct (P3)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 5.5: advisor-lot-realism

As the operator of the Honest Advisor,
I want opt-in min-notional + lot-size realism at the universal fill chokepoint (default byte-identical; day-1 divergence e2e) (P4),
so that the shipped product is provably done, with its thesis boundary honestly mapped.

**Acceptance Criteria:**

**Given** the repo history at `advisor-lot-realism`'s landing commits (`git log -- spec/v3/advisor-lot-realism`)
**When** the recorded verification for `advisor-lot-realism` is replayed (tests, reports under `spec/v3/advisor-lot-realism/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: opt-in min-notional + lot-size realism at the universal fill chokepoint (default byte-identical; day-1 divergence e2e) (P4)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 5.6: advisor-handoff-export

As the operator of the Honest Advisor,
I want the deterministic plan export (plan-{coin}-{window}-{seed8}.md, operator-ratified golden-locked wording incl. the short unbounded-loss case) (P5),
so that the shipped product is provably done, with its thesis boundary honestly mapped.

**Acceptance Criteria:**

**Given** the repo history at `advisor-handoff-export`'s landing commits (`git log -- spec/v3/advisor-handoff-export`)
**When** the recorded verification for `advisor-handoff-export` is replayed (tests, reports under `spec/v3/advisor-handoff-export/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the deterministic plan export (plan-{coin}-{window}-{seed8}.md, operator-ratified golden-locked wording incl. the short unbounded-loss case) (P5)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

## Epic 6: Remediation, Infra & Governance (P0-P8, lints, BMAD migration)

Core infrastructure (reflection memory, operator reports, audit envelope, paper-soak) and the governance/tooling layer (schema/registry/staleness lints, anchors + spec-lint gates), plus cross-platform CI activation and the live BMAD-method migration.

### Story 6.1: reflection-memory

As the operator of the Honest Advisor,
I want the persistent lesson-card store with retrieval at decision time, wired through the sanctioned ADR-0041 layering seam (+ trader wiring),
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

**Acceptance Criteria:**

**Given** the repo history at `reflection-memory`'s landing commits (`git log -- spec/v1/reflection-memory`)
**When** the recorded verification for `reflection-memory` is replayed (tests, reports under `spec/v1/reflection-memory/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the persistent lesson-card store with retrieval at decision time, wired through the sanctioned ADR-0041 layering seam (+ trader wiring)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 6.2: operator-success-reports

As the operator of the Honest Advisor,
I want auto-generated "is this working?" operator reports (equity, Sharpe/Sortino/drawdown, attribution, system health),
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

**Acceptance Criteria:**

**Given** the repo history at `operator-success-reports`'s landing commits (`git log -- spec/v1/operator-success-reports`)
**When** the recorded verification for `operator-success-reports` is replayed (tests, reports under `spec/v1/operator-success-reports/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: auto-generated "is this working?" operator reports (equity, Sharpe/Sortino/drawdown, attribution, system health)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 6.3: audit-tick-consumer-envelope

As the operator of the Honest Advisor,
I want the audit tick consumer with an aggregation envelope,
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

**Acceptance Criteria:**

**Given** the repo history at `audit-tick-consumer-envelope`'s landing commits (`git log -- spec/v1/audit-tick-consumer-envelope`)
**When** the recorded verification for `audit-tick-consumer-envelope` is replayed (tests, reports under `spec/v1/audit-tick-consumer-envelope/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the audit tick consumer with an aggregation envelope
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 6.4: paper-soak-longevity

As the operator of the Honest Advisor,
I want the reflection-loop paper wiring (lesson card per closed trade) + the longevity soak evidence artifact + the 90-day soak runbook,
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

**Acceptance Criteria:**

**Given** the repo history at `paper-soak-longevity`'s landing commits (`git log -- spec/v1/paper-soak-longevity`)
**When** the recorded verification for `paper-soak-longevity` is replayed (tests, reports under `spec/v1/paper-soak-longevity/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the reflection-loop paper wiring (lesson card per closed trade) + the longevity soak evidence artifact + the 90-day soak runbook
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 6.5: operator-ledger-schema-lint

As the operator of the Honest Advisor,
I want the ledger chart-of-accounts schema lint,
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

**Acceptance Criteria:**

**Given** the repo history at `operator-ledger-schema-lint`'s landing commits (`git log -- spec/v1/operator-ledger-schema-lint`)
**When** the recorded verification for `operator-ledger-schema-lint` is replayed (tests, reports under `spec/v1/operator-ledger-schema-lint/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the ledger chart-of-accounts schema lint
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 6.6: adr-registry-atomic-lint

As the operator of the Honest Advisor,
I want the ADR-registry atomicity lint (sibling pre-commit guard),
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

**Acceptance Criteria:**

**Given** the repo history at `adr-registry-atomic-lint`'s landing commits (`git log -- spec/v1/adr-registry-atomic-lint`)
**When** the recorded verification for `adr-registry-atomic-lint` is replayed (tests, reports under `spec/v1/adr-registry-atomic-lint/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the ADR-registry atomicity lint (sibling pre-commit guard)
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 6.7: queue-staleness-reconciliation

As the operator of the Honest Advisor,
I want the backlog-queue staleness reconciliation pass,
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

**Acceptance Criteria:**

**Given** the repo history at `queue-staleness-reconciliation`'s landing commits (`git log -- spec/v1/queue-staleness-reconciliation`)
**When** the recorded verification for `queue-staleness-reconciliation` is replayed (tests, reports under `spec/v1/queue-staleness-reconciliation/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the backlog-queue staleness reconciliation pass
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 6.8: subscription-pipe-server-time-template

As the operator of the Honest Advisor,
I want the server-time template closing the Wave-1 subscription-pipe carve-out,
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

**Acceptance Criteria:**

**Given** the repo history at `subscription-pipe-server-time-template`'s landing commits (`git log -- spec/v1/subscription-pipe-server-time-template`)
**When** the recorded verification for `subscription-pipe-server-time-template` is replayed (tests, reports under `spec/v1/subscription-pipe-server-time-template/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the server-time template closing the Wave-1 subscription-pipe carve-out
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 6.9: cockpit-cross-platform

As the operator of the Honest Advisor,
I want the cockpit on Linux/Windows: source shipped + macOS-verified; the 3-OS CI matrix ACTIVATED 2026-07-10 (P7) - the run-2 shakeout is the open in-progress work,
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

**Acceptance Criteria:**

**Given** the activated 3-OS CI matrix (ci.yml live on push/PR) with run-2 shakeout reds open
**When** the shakeout fixes land (fix-forward per the operator direction)
**Then** the Linux/Windows lanes go green and the story flips to done - until then it is honestly in-progress
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 6.10: bmad-method-migration

As the operator of the Honest Advisor,
I want the operator-ratified full migration to BMAD-METHOD v6.10.0 (7 phases; Phase 0 install + Phase 1 planning docs landed; THIS story - Phase 2 retro epics/stories/sprint-status - is the live work; Phases 3-5c pending),
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

**Acceptance Criteria:**

**Given** the ratified migration plan and the Phase 0/1 commits on main
**When** Phases 2-5c execute (epics/stories, corpus move + anchor base-swap, knowledge move, personas, lint re-founding, docs cutover) with gates green at every commit
**Then** spec/ is retired with zero guarantees dropped: verify_anchors 119/119, re-founded spec_lint PASS, trace ledger authoritative at its new path
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

## Epic 7: Retired Research Lines (measured-and-retired bets)

The honest negative-result record: the v2.5 DL forecaster programme, the v3 forecaster/classifier bets, and the no-op-overlay precedent. Code + anchors retained; the lines are retired, and staying retired is a product guarantee (do-not-build register).

### Story 7.1: v25-dl-forecast-overlay

As the operator of the Honest Advisor,
I want the v2.5 DL forecast overlay 4-phase roadmap (the programme umbrella) - terminal F4 across two model families; RETIRED 2026-05-22,
so that the measured dead-ends stay on the record so they are never re-litigated.

**Acceptance Criteria:**

**Given** the repo history at `v25-dl-forecast-overlay`'s landing commits (`git log -- spec/v1/v25-dl-forecast-overlay`)
**When** the recorded verification for `v25-dl-forecast-overlay` is replayed (tests, reports under `spec/v1/v25-dl-forecast-overlay/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the v2.5 DL forecast overlay 4-phase roadmap (the programme umbrella) - terminal F4 across two model families; RETIRED 2026-05-22
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 7.2: v25-tcn-overlay

As the operator of the Honest Advisor,
I want the TCN forecast overlay (phase 1 of 4) + its alpha-investigation / recalibrate / threshold-tuning / horizon-bump sub-studies - no +0.10 Sharpe delta; line retired,
so that the measured dead-ends stay on the record so they are never re-litigated.

**Acceptance Criteria:**

**Given** the repo history at `v25-tcn-overlay`'s landing commits (`git log -- spec/v1/v25-tcn-overlay`)
**When** the recorded verification for `v25-tcn-overlay` is replayed (tests, reports under `spec/v1/v25-tcn-overlay/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the TCN forecast overlay (phase 1 of 4) + its alpha-investigation / recalibrate / threshold-tuning / horizon-bump sub-studies - no +0.10 Sharpe delta; line retired
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 7.3: v25a-patchtst-overlay

As the operator of the Honest Advisor,
I want the PatchTST forecast overlay (phase 2 of 4) - null; line retired,
so that the measured dead-ends stay on the record so they are never re-litigated.

**Acceptance Criteria:**

**Given** the repo history at `v25a-patchtst-overlay`'s landing commits (`git log -- spec/v1/v25a-patchtst-overlay`)
**When** the recorded verification for `v25a-patchtst-overlay` is replayed (tests, reports under `spec/v1/v25a-patchtst-overlay/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the PatchTST forecast overlay (phase 2 of 4) - null; line retired
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 7.4: v25b-transformer-overlay

As the operator of the Honest Advisor,
I want the vanilla decoder-only Transformer overlay (phase 3 of 4) - null; deprecated,
so that the measured dead-ends stay on the record so they are never re-litigated.

**Acceptance Criteria:**

**Given** the repo history at `v25b-transformer-overlay`'s landing commits (`git log -- spec/v1/v25b-transformer-overlay`)
**When** the recorded verification for `v25b-transformer-overlay` is replayed (tests, reports under `spec/v1/v25b-transformer-overlay/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the vanilla decoder-only Transformer overlay (phase 3 of 4) - null; deprecated
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 7.5: v26-forecast-bakeoff

As the operator of the Honest Advisor,
I want the v2.6 forecast bake-off + retirement decision (phase 4 of 4) closing the DL programme,
so that the measured dead-ends stay on the record so they are never re-litigated.

**Acceptance Criteria:**

**Given** the repo history at `v26-forecast-bakeoff`'s landing commits (`git log -- spec/v1/v26-forecast-bakeoff`)
**When** the recorded verification for `v26-forecast-bakeoff` is replayed (tests, reports under `spec/v1/v26-forecast-bakeoff/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the v2.6 forecast bake-off + retirement decision (phase 4 of 4) closing the DL programme
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 7.6: v3-llm-forecaster

As the operator of the Honest Advisor,
I want the v3 LLM-as-forecaster (reflection-memory + audit-trail-anchored signal) - shipped-partial: the alpha-verdict wave deferred on absent ANTHROPIC_API_KEY,
so that the measured dead-ends stay on the record so they are never re-litigated.

**Acceptance Criteria:**

**Given** the repo history at `v3-llm-forecaster`'s landing commits (`git log -- spec/v1/v3-llm-forecaster`)
**When** the recorded verification for `v3-llm-forecaster` is replayed (tests, reports under `spec/v1/v3-llm-forecaster/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the v3 LLM-as-forecaster (reflection-memory + audit-trail-anchored signal) - shipped-partial: the alpha-verdict wave deferred on absent ANTHROPIC_API_KEY
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 7.7: v3-volatility-forecaster

As the operator of the Honest Advisor,
I want the v3 GARCH-sigma volatility forecaster (predict sigma, not mu) + the noop-fix that exposed the computed-but-unapplied overlay - MODEL-BROKEN / NO-ALPHA; retired,
so that the measured dead-ends stay on the record so they are never re-litigated.

**Acceptance Criteria:**

**Given** the repo history at `v3-volatility-forecaster`'s landing commits (`git log -- spec/v1/v3-volatility-forecaster`)
**When** the recorded verification for `v3-volatility-forecaster` is replayed (tests, reports under `spec/v1/v3-volatility-forecaster/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the v3 GARCH-sigma volatility forecaster (predict sigma, not mu) + the noop-fix that exposed the computed-but-unapplied overlay - MODEL-BROKEN / NO-ALPHA; retired
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 7.8: v3-volatility-forecaster-rebaseline

As the operator of the Honest Advisor,
I want the v3 volatility-forecaster re-baseline pass confirming the retirement verdict,
so that the measured dead-ends stay on the record so they are never re-litigated.

**Acceptance Criteria:**

**Given** the repo history at `v3-volatility-forecaster-rebaseline`'s landing commits (`git log -- spec/v1/v3-volatility-forecaster-rebaseline`)
**When** the recorded verification for `v3-volatility-forecaster-rebaseline` is replayed (tests, reports under `spec/v1/v3-volatility-forecaster-rebaseline/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the v3 volatility-forecaster re-baseline pass confirming the retirement verdict
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 7.9: v3-regime-classifier

As the operator of the Honest Advisor,
I want the v3 regime classifier (predict regime label, not mu) - foreclosed when the OHLCV channel was exhausted,
so that the measured dead-ends stay on the record so they are never re-litigated.

**Acceptance Criteria:**

**Given** the repo history at `v3-regime-classifier`'s landing commits (`git log -- spec/v1/v3-regime-classifier`)
**When** the recorded verification for `v3-regime-classifier` is replayed (tests, reports under `spec/v1/v3-regime-classifier/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the v3 regime classifier (predict regime label, not mu) - foreclosed when the OHLCV channel was exhausted
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 7.10: v3-xgboost-cheap-classifier

As the operator of the Honest Advisor,
I want the v3 XGBoost cheap classifier (low-capacity regime label on hourly OHLCV) - foreclosed; retired,
so that the measured dead-ends stay on the record so they are never re-litigated.

**Acceptance Criteria:**

**Given** the repo history at `v3-xgboost-cheap-classifier`'s landing commits (`git log -- spec/v1/v3-xgboost-cheap-classifier`)
**When** the recorded verification for `v3-xgboost-cheap-classifier` is replayed (tests, reports under `spec/v1/v3-xgboost-cheap-classifier/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the v3 XGBoost cheap classifier (low-capacity regime label on hourly OHLCV) - foreclosed; retired
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

### Story 7.11: vol-killswitch-overlay-noop-fix

As the operator of the Honest Advisor,
I want the fix for the computed-but-unapplied vol kill-switch overlay - the precedent behind the day-1 baseline-equity-divergence e2e non-negotiable,
so that the measured dead-ends stay on the record so they are never re-litigated.

**Acceptance Criteria:**

**Given** the repo history at `vol-killswitch-overlay-noop-fix`'s landing commits (`git log -- spec/v1/vol-killswitch-overlay-noop-fix`)
**When** the recorded verification for `vol-killswitch-overlay-noop-fix` is replayed (tests, reports under `spec/v1/vol-killswitch-overlay-noop-fix/reports/` where present, render proofs where UI-facing)
**Then** the shipped behaviour holds: the fix for the computed-but-unapplied vol kill-switch overlay - the precedent behind the day-1 baseline-equity-divergence e2e non-negotiable
**And** the standing floor holds: `verify_anchors` 119/119, `spec_lint` PASS.

