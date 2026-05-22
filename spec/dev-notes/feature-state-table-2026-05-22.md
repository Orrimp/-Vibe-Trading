---
slug: feature-state-table-2026-05-22
date: 2026-05-22
authors: orchestrator
status: proposed
related:
  - spec/backlog.md
  - spec/dev-notes/repo-cleanup-plan-2026-05-22.md
  - spec/dev-notes/retired-surface-inventory-2026-05-22.md
  - spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md
  - spec/dev-notes/v3-vol-retirement-and-c5-promotion-2026-05-22.md
---

# Feature state table — 2026-05-22

Comprehensive inventory of all 54 active feature folders under `spec/`. Operator-requested revisit at session-end. Columns: state, version, last-updated, purpose (1 sentence), outcome / notes.

## Status legend

| State | Meaning |
|---|---|
| **shipped** | Code on `main`, anchors locked, tests green. Continues to be load-bearing. |
| **shipped-partial** | Code gates clean; one wave deferred for external-dependency reasons (e.g. API key). First used 2026-05-22 by `v3-llm-forecaster`. |
| **retired** | Research line closed. Code stays in tree; anchors locked; no further effort. |
| **deprecated** | Roadmap item never built (or built and superseded). |
| **draft** | Analyst-only spec; no code commitment. |
| **proposed** | Brief authored + reviewed; waiting on operator decision. |
| **candidate** | Under evaluation; may or may not promote. |
| **roadmap** | Multi-phase initiative under planning (Lumen design system). |

## Group A — Strategy research (shipped + retired)

The "did we find alpha?" lane. 13 features. Currently: 0 active research, 3 retired forecasters, 1 partial ship, 1 draft. The shipped strategies (v0/v05/v1/v15a/v1.5b/v2) are evidence; the forecaster experiments (v25*/v3*) are research history.

| Slug | State | Version | Updated | Purpose | Outcome |
|---|---|---|---|---|---|
| `v0-paper-sma` | shipped | 0.1.0 | 2026-05-16 | First strategy ship — SMA crossover paper-trade baseline | Anchored evidence; load-bearing baseline for later comparisons |
| `v05-composed-strategies` | shipped | 0.5.0 | 2026-05-16 | Strategy composition framework (multiple signals merged via builder) | Active infrastructure; `with_*` composition pattern reused by overlays |
| `v1-cross-sectional-momentum` | shipped | 1.0.0 | 2026-05-16 | v1 momentum on top-10 crypto universe; hourly rebalance | THE production baseline; ~13% return on 2023-FY; all subsequent overlays measured vs this |
| `v15a-mean-reversion-pairs` | shipped | 1.1.0 | 2026-04-29 | Mean-reversion pairs trading framework | Anchored; in tree |
| `v1-5b-multi-venue` | shipped | 1.2.0 | 2026-05-03 | Multi-venue execution support | Anchored; in tree |
| `v2-llm-strategy` | shipped | 2.0.0 | 2026-05-13 | LLM-as-analyst infrastructure (LlmProvider trait + Recording/Replay + BudgetedProvider) | Foundation reused by `v3-llm-forecaster`; ships replay-cache determinism contract |
| `v25-dl-forecast-overlay` | **deprecated** | 2.5.0 | 2026-05-17 | Umbrella for v2.5 DL forecast overlays (TCN + PatchTST + transformer + bakeoff) | Programme-retired 2026-05-22 after joint F4-F4-F4 verdict; see retrospective dev-note |
| `v25-tcn-overlay` | shipped | 2.5.0 | 2026-05-22 | TCN forecast overlay v0.0.0 → v0.1.0 (now historical) | F-verdict F4; retired; see `v25-tcn-alpha-investigation` for evidence |
| `v25-tcn-alpha-investigation` | shipped | 0.3.0 | 2026-05-19 | F-verdict bin + per-symbol QLIKE + investigation report shape | ADR-0033 § D3 (IMMUTABLE F-verdict) defined here |
| `v25-tcn-recalibrate` | shipped | 0.1.0 | 2026-05-21 | σ_train post-hoc recalibration (ADR-0035 fix for the 608×/580× inflation bug) | Confounding variable removed; gate-survival jumped (BS-1 τ=0.6: 0% → 40.1%) but F-verdict stayed F4 |
| `v25-tcn-threshold-tuning` | shipped | 0.1.0 | 2026-05-21 | τ-sweep on recalibrated TCN to find marginal alpha | Joint T-MARGINAL (BS-1 +0.018 / BS-2 +0.045) — below T-ALPHA-UNLOCKED but not zero |
| `v25-tcn-horizon-bump-or-retire` | shipped | 0.1.0 | 2026-05-21 | Operator-decision feature: bump TCN horizon vs retire for PatchTST | Operator chose (b) RETIRE; multi-week budget pivoted to PatchTST |
| `v25a-patchtst-overlay` | shipped | 0.1.0 | 2026-05-21 | Patch-attention transformer forecast overlay (PatchTST architecture) | F-verdict F4 + Sharpe-delta only +0.006144; weaker than recalibrated TCN; helped retire the whole programme |
| `v25b-transformer-overlay` | **deprecated** | 2.5.2 | 2026-05-17 | Roadmap-only; never built | Programme-retired with parent v25-dl |
| `v26-forecast-bakeoff` | **deprecated** | 2.6.0 | 2026-05-17 | Roadmap-only; would have been ensemble bakeoff across TCN/PatchTST/etc | Programme-retired |
| `v3-volatility-forecaster` | **retired** | 0.1.0 | 2026-05-22 | GARCH(1,1) vol forecasting → vol-targeting overlay (C1 of post-DL reformulation) | RETIRED after noop-fix revealed MODEL-BROKEN / NO-ALPHA / NEGATIVE-NET-DELTA under real wiring |
| `v3-volatility-forecaster-rebaseline` | **retired** | 0.1.0 | 2026-05-22 | Re-baseline pass per operator (b) routing — compare vs REAL un-targeted v1 baseline | RETIRED with parent; the rebaseline was correct conclusion (NO-ALPHA) fortuitously — noop-fix later revealed the underlying bug |
| `v3-volatility-forecaster-noop-fix` | shipped | 0.1.0 | 2026-05-22 | P0 wiring-bug fix: scale computed but never applied to fill quantities | Fixed via `Strategy::quantity_scale` trait method + R2 forensic regression test; ADR-0038 § D6.b re-emission protocol amendment |
| `v3-llm-forecaster` | **shipped-partial** | 0.1.0 | 2026-05-22 | LLM-as-forecaster (C5 of post-DL reformulation); moat-aligned with product.md § Differentiator | 6 of 7 waves shipped; Wave D (backtest + real API + canonical cache) DEFERRED indefinitely per operator (no ANTHROPIC_API_KEY this session) |
| `v3-regime-classifier` | draft | 0.1.0 | 2026-05-22 | Regime classifier (C2 of post-DL reformulation) — multi-symbol extension of `crates/reflection/src/regime.rs` 3-state BTC tagger | Queue-only; not yet promoted |

## Group B — Backtest + audit + data infra

Load-bearing infrastructure. 6 features. All shipped.

| Slug | State | Version | Updated | Purpose | Outcome |
|---|---|---|---|---|---|
| `backtest-real-binance-data` | shipped | 0.1.0 | 2026-05-18 | Real Binance OHLCV path for backtest (ADR-0032 revision pin) | All -realdata scenarios derive from this; dataset SHA `3a8b96c43f...` pinned |
| `journal-transactions-metadata` | shipped | 1.6.1 | 2026-05-03 | Double-entry journal transaction metadata extensions | Audit ledger backbone; reused by every strategy emit |
| `per-symbol-position-accounts` | shipped | 1.4.0 | 2026-05-03 | Per-symbol accounting (vs commingled) | Migration 006 in `crates/audit/migrations/` |
| `real-mtm-unrealized-pnl` | shipped | 1.3.0 | 2026-05-02 | Mark-to-market unrealized P&L at audit-tick boundaries | Cost-tracking foundation |
| `reflection-memory` | shipped | 1.8.0 | 2026-05-08 | Persistent reflection memory + LessonCard store + top_k retrieval (`crates/reflection`) | Product.md moat surface (2); reused by `v3-llm-forecaster` C5 + `v3-regime-classifier` C2 |
| `audit-tick-consumer-envelope` | shipped | 0.1.0 | 2026-05-20 | Audit-tick consumer interface envelope | Decouples tick emit from downstream consumers |

## Group C — Cockpit / live trading

Operator-facing surface. 5 features.

| Slug | State | Version | Updated | Purpose | Outcome |
|---|---|---|---|---|---|
| `live-cockpit-unified` | shipped | 1.5.0 | 2026-05-02 | Unified cockpit binary for live + backtest views | THE entry-point bin; `cockpit_live` runs the iced app |
| `cockpit-app-bundle` | **candidate** | 0.1.0 | 2026-05-11 | macOS .app bundle + signing | Under evaluation; not promoted |
| `cockpit-performance-and-input-responsiveness` | shipped | 1.0.0 | 2026-05-15 | Cockpit perf audit + input-latency budget | Frame-budget invariants codified |
| `cockpit-render-regression` | shipped | 1.0.0 | 2026-05-14 | Render-regression test gate | Visual snapshot baselines anchored |
| `cockpit-training-control` | shipped | 0.2.0 | 2026-05-19 | Cockpit training Start/Pause/Resume controls | Training pipeline UI integration |
| `operator-success-reports` | shipped | 1.7.0 | 2026-05-01 | LLM-generated operator success reports on demand | Uses v2-llm-strategy infrastructure; the analyst LLM call site |

## Group D — UI rethink (6-phase initiative, complete)

The 2026-05-17 → 2026-05-21 UI overhaul. Closed cleanly; final phase shipped 2026-05-21.

| Slug | State | Version | Updated | Purpose | Outcome |
|---|---|---|---|---|---|
| `ui-rethink-phase-a-lab` | shipped | 0.2.0 | 2026-05-18 | Phase A — Lab screen (strategy experimentation) | Foundation; subsequent phases compose with this |
| `ui-rethink-phase-b-lab-run` | shipped | 0.2.0 | 2026-05-19 | Phase B — Run button + execution engine for Lab | Lab can now produce live runs |
| `ui-rethink-phase-c-sidebar-ia` | shipped | 0.1.0 | 2026-05-20 | Phase C — Sidebar IA flip + Live + Strategy registry | Information architecture redesign |
| `ui-rethink-phase-d-trail` | shipped | 0.1.0 | 2026-05-20 | Phase D — Trail view (audit-trail visualization) | Trail screen ships |
| `ui-rethink-phase-d-trail-followup` | shipped | 0.1.1 | 2026-05-20 | Phase D+ — Trail view polish + forecast_context wiring | T-D-N5 follow-up |
| `ui-rethink-phase-e-compare` | shipped | 0.1.0 | 2026-05-20 | Phase E — Compare screen (strategy A vs B) | Side-by-side comparison surface |
| `ui-rethink-phase-f-memory-models-assistant` | shipped | 0.1.0 | 2026-05-20 | Phase F — Memory + Models + Assistant slot (placeholder body) | Final phase; Assistant slot body promoted to LLM-forecaster reasoning in `v3-llm-forecaster` Wave F |

## Group E — Chart / canvas

Chart rendering subsystem. 4 features.

| Slug | State | Version | Updated | Purpose | Outcome |
|---|---|---|---|---|---|
| `chart-buy-sell-emphasis` | shipped | 1.9.0 | 2026-05-11 | Buy/sell marker emphasis on chart canvas | Visual polish |
| `chart-canvas-overhaul` | shipped | 1.10.0 | 2026-05-12 | Chart canvas rewrite (iced-native draw API) | Foundation for subsequent chart ships |
| `chart-fixture-line-clipping` | shipped | 1.0.0 | 2026-05-20 | Line-clipping fix in vendored `iced_tiny_skia` | Operator-locked vendor fork; CLAUDE.md documents maintenance contract |
| `chart-x-axis-local-time` | shipped | 1.11.0 | 2026-05-20 | Local-time x-axis labels (vs UTC) | Per-OS offset wired |

## Group F — UI infrastructure / testing

iced ecosystem + test infrastructure. 10 features.

| Slug | State | Version | Updated | Purpose | Outcome |
|---|---|---|---|---|---|
| `iced-aw-cherry-pick` | shipped | 1.0.0 | 2026-05-16 | Cherry-pick subset of iced_aw widgets | Reduced footprint; superseded by `ui-drop-iced-aw` |
| `ui-drop-iced-aw` | shipped | 0.1.0 | 2026-05-16 | Drop iced_aw dep entirely | All custom widgets now in-tree |
| `iced-ecosystem-evaluation` | **candidate** | 0.2.0 | 2026-05-13 | Evaluate iced 0.14 vs alternatives | Operator-locked at iced 0.14.0; CLAUDE.md vendor lock |
| `iced-native-widgets` | shipped | 0.1.0 | 2026-05-13 | In-tree iced-native widget shims | Reused throughout cockpit |
| `ui-gallery-bin` | **shipped-partial-terminal** | 0.1.0-partial-terminal | 2026-05-16 | Widget gallery binary for visual regression | Terminal-mode only; V5+ blocked on upstream iced bug |
| `ui-gallery-table-cell` | draft | — | 2026-05-16 | In-tree table-cell workaround | Unblocks ui-gallery-bin pending upstream fix |
| `ui-headless-emulator` | shipped | 0.1.0 | 2026-05-16 | Headless iced emulator for snapshot testing | Visual snapshot infrastructure |
| `ui-quality-gate-overhaul` | shipped | 1.0.0 | 2026-05-15 | Overhauled UI quality gates (clippy + visual + layout + proptests) | Tester contract |
| `ui-session-journal-iced-tester` | shipped | 0.1.0 | 2026-05-16 | Per-session ui journal + iced-tester harness | Replay-based test infrastructure |
| `ui-test-harness-bootstrap` | shipped | 0.1.0 | 2026-05-12 | Bootstrap shape for the UI test harness | Foundation |

## Group G — Design system + misc

| Slug | State | Version | Updated | Purpose | Outcome |
|---|---|---|---|---|---|
| `lumen-design-adoption` | **roadmap** | 2.0.0 | 2026-05-04 | Lumen design system roadmap (multi-phase) | Active design language; tokens are project-wide |
| `tape-row-audit-modal` | shipped | 1.6.0 | 2026-05-03 | Audit-tick row modal in tape view | Audit-trail UI surface |

## Summary stats

| Category | Count | Notes |
|---|---|---|
| **shipped** | 36 | Code on main; load-bearing |
| **shipped-partial** | 2 | `v3-llm-forecaster` v0.1.0 + `ui-gallery-bin` v0.1.0-partial-terminal |
| **retired** | 2 | `v3-volatility-forecaster` + rebaseline |
| **deprecated** | 3 | `v25-dl-forecast-overlay` umbrella + `v25b-transformer-overlay` + `v26-forecast-bakeoff` (programme-retired) |
| **draft** | 2 | `v3-regime-classifier` + `ui-gallery-table-cell` |
| **proposed** | 0 | (None — all promoted or retired) |
| **candidate** | 3 | `cockpit-app-bundle` + `iced-ecosystem-evaluation` + (others) |
| **roadmap** | 1 | `lumen-design-adoption` |
| **Total** | 54 | + 6 archived dev-notes; + 1 P0 fix |

## What this tells you about the project

1. **The forecaster research track is exhausted**. v25-dl umbrella + 4 children + v3-vol + rebaseline all retired. v3-llm shipped-partial; v3-regime drafted but not promoted. The strategy-reformulation HYBRID picked 3 candidates (C1/C2/C5); 1 retired with negative evidence, 1 shipped partial with deferred verdict, 1 not promoted.

2. **The UI track is essentially complete**. All 6 rethink phases shipped; Lumen design system stable; chart subsystem stable; test infrastructure mature.

3. **Live trading is the unfilled gap**. `live-cockpit-unified` v1.5.0 exists, but no feature folder for "v1 momentum goes live" exists. This is the largest EV-per-effort unfilled slot.

4. **Infrastructure is solid**. Audit ledger / journal / per-symbol accounting / reflection-memory / replay-cache all shipped + load-bearing. The v3-llm noop-fix shows the test-coverage pattern is working (R2 forensic regression tests catch wiring bugs).

5. **Spec-driven workflow is mature**. 54 features in ~3 weeks of activity; consistent frontmatter; clean retirement protocols; the precedent-setting `shipped-partial` state codifies external-dependency-deferral as a sanctioned ship state.

## Open candidate features (for operator routing)

- **`v3-regime-classifier` (C2 of strategy-reformulation)** — still in Queue. ~4-6 weeks. Lower novelty than C5; existing seed in `crates/reflection/src/regime.rs`.
- **`v3-llm-forecaster` v0.1.1 (Wave D)** — paused indefinitely. Requires `ANTHROPIC_API_KEY` + ~$25-50 spend + ~half-day work.
- **Paper-trade-live for v1 momentum** — NO feature folder yet. The v1 cross-sectional momentum strategy has positive Sharpe across all 4 retire-chain comparisons but has never been put in front of live order flow.
- **`cockpit-app-bundle`** — candidate for ages; could promote if operator wants distributable cockpit.

## Cross-references

- `spec/backlog.md` — the canonical Active/Queue/Recent split (this dev-note is a flattened snapshot).
- `spec/dev-notes/retired-surface-inventory-2026-05-22.md` — code-level inventory of retired features.
- `spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md` — the DL forecaster retirement narrative.
- `spec/dev-notes/v3-vol-retirement-and-c5-promotion-2026-05-22.md` — the most-recent retirement chain.
- `spec/dev-notes/strategy-reformulation-survey-2026-05-22.md` — the C1/C2/C5 survey that drove the v3 track.
- `spec/dev-notes/repo-cleanup-plan-2026-05-22.md` — current cleanup plan.
