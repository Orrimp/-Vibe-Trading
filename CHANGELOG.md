# Changelog

The canonical **"what's been built"** index for this Rust crypto-trading agent —
one line per implemented feature, grouped by subsystem. The project runs several
parallel version ladders (strategy/engine `v0…v5`, plus independently-versioned
cockpit/UI and infra tracks), so entries are grouped by subsystem and tagged with
their own version rather than forced onto a single release timeline.

Per-feature narrative history lives in **git** (`git log -- spec/<slug>/`). The
immutable backtest evidence lives under **`spec/*/reports/`** (119 byte-SHA-256
regression anchors, gated by `scripts/verify_anchors.sh`). Ratified scope and
design remain in **`spec/product.md`**, **`spec/architecture/`** (ADRs), and
**`spec/runbooks/`**.

> **Program status — single-coin investment advisor (paper), MVP SHIPPED (2026-06-19 pivot).**
> The terminal deliverable is the **advisor loop**: pick a coin + budget (e.g. €200
> XRPUSDT) → bake off *every* strategy over a configurable 2-week-to-4-year window →
> rank & select the best by risk-adjusted Sharpe under a robustness gate → emit a
> forward buy/sell plan → **paper-trade the budget forward** in the Live view with
> real P/L. The 2026-06-08 active-vs-passive negative result (no active strategy beat
> passive buy-and-hold net of cost under the frozen block-bootstrap rule) is REFRAMED,
> not discarded: **buy-and-hold is the always-present bake-off benchmark arm** and the
> **Monte-Carlo robustness machine is the credibility layer** that gates every pick —
> the foundation under the advisor, not the terminal product. PAPER-ONLY (simulated
> fills, simulated €200); not financial advice; single-coin. **No live trading**
> (removed 2026-06-12, out of scope). Full spec: `spec/product.md`.

---

## Single-coin investment advisor (paper) — 2026-06-19 pivot, MVP SHIPPED

The terminal product: pick coin + budget → bake off all strategies → rank the best → forward paper-trade the €200. Buy-and-hold is the always-present benchmark arm; the Monte-Carlo robustness machine gates every pick.

- **advisor-bakeoff F1+F2** (v0.1.0) — strategy bake-off + ranking engine (`crates/backtest/src/bakeoff/`: `run_bakeoff(cfg) -> BakeoffReport`; ranks Fragile-ineligible → Sharpe → return → drawdown → id; buy-and-hold always the benchmark arm; structured `Recommendation`). ADR-0059. `58b55b1`.
- **advisor-leaderboard** (v0.1.0) — cockpit Leaderboard screen rendering the ranked bake-off (advisor journey step 3) — `crates/ui/src/screens/leaderboard.rs`, `crates/ui/src/leaderboard/`. `e0cc34b`.
- **advisor-bakeoff F3** (v0.1.0) — guided coin + budget + lookback input widget (`crates/ui/src/widgets/bakeoff_input.rs`) opening the journey. `acc3789`.
- **advisor-forward-paper F4** (v0.2.0) — budget-aware €200 sizing modifier (`crates/risk` `FixedFractionSizer.budget_cap`) with a day-1 baseline-equity-divergence e2e (`crates/risk/tests/budget_sizing_divergence_end_to_end.rs`). ADR-0060. `d4f4dce`.
- **advisor-forward-paper F5** (v0.2.0) — forward paper-trade of the SELECTED strategy: `crates/agent::runtime` `paper_loop_supervisor` hot-swaps the trading loop on `ForwardCommand::Launch` to run the pick at €200 on the same bus/ledger; Live view shows real P/L. ADR-0060 §D6. `c9dd275`.
- **advisor-dynamic-data** (v0.1.0) — on-demand Binance fetch for any coin + window the pinned corpus doesn't cover (`crates/data` `binance_klines` + `dynamic_cache`, git-ignored cache root, anchor-safe by construction — verify_anchors stays 119/119). ADR-0061. `ee5a904`.
- **advisor-forward-fidelity F5b** (v0.2.0) — the forward run executes the REAL crowned strategy, not an SMA proxy: `build_registry_for` loads the actual `ComposedStrategy` (MACD/RSI/Bollinger from `config/strategies/*.toml`) + a new `strategy::AlwaysLongStrategy` for buy-and-hold, unknown id → typed `Err` (no silent fallback); anti-fake gate `crates/agent/tests/forward_run_engine_fidelity.rs` (registry-identity + MACD≠SMA divergence). `608cf1c`.
- **advisor-forward-plan F6** (v0.1.0) — the honest, conditional forward buy/sell plan (new Plan screen between Leaderboard and Live): read-only `PlanDescribe` seam → core-typed `ForwardPlan` → `ForwardPlanView`; IF/THEN rules faithful to the real TOMLs (flip-to-false exits, no fabricated thresholds) with not-a-prediction/not-advice framing; configurable 1–30d horizon (default 7). ADR-0062. `51aba16`.

## Strategy & backtest engine

- **v0** — Paper-trading SMA-crossover tracer bullet: end-to-end harness (core types, Binance data, audit ledger, cockpit) proving a trivial strategy round-trips with reconciling double-entry books.
- **v0.5** — Composed strategies (hot-loadable TOML indicator/rule assemblies, atomic swap on file change) plus the multi-indicator rule set (MACD + RSI + Bollinger).
- **v1** — Cross-sectional top-N momentum: first multi-symbol real-edge candidate.
- **v1.5a** — Mean-reversion on z-scored pairs (pairs/portfolio plumbing).
- **v1.5b** — Multi-venue support + 1-second aggregated trades.
- **v2** — LLM news/sentiment strategy overlay (first LLM-in-the-loop strategy; support layer, not the alpha source).
- **v2.1** — tracing-`Layer` secret redactor wired across 17 binaries (`REDACT_LAYER_MODE`, WARN default).
- **v5** — Deterministic latency & slippage simulation closing the backtest-vs-live gap, landed across `v0.1`→`v0.5`: canonical medium-friction model (`slippage_bps: 8`, square-root market impact) with the full anchor-migration chain (v0.2 anchor migration, v0.3 full-path wiring, v0.4 candle/realdata feature-gated re-emit, v0.5 sqrt-impact).
- **backtest-real-binance-data** — Real-Binance-data backtest path (pinned corpus, deterministic seeds).
- **simple-strategies-realdata** — SMA / MACD / RSI / Bollinger runnable on real Binance data in the Lab.

## Robustness program — CONCLUDED 2026-06-08 → ship passive

- **monte-carlo-bootstrap-path-generator** (C1) — stationary-block-bootstrap path generator that resamples real returns (Politis–White auto block length), preserving fat tails and volatility clustering.
- **strategy-robustness-harness** (C2) — distribution-summary backtest mode (Sharpe p5/p50/p95, max-drawdown tail, probability-of-loss) read against the frozen § 0 decision rule (p5 Sharpe < 0 → FRAGILE).
- **momentum-parameter-robustness-sweep** — distribution-per-θ over the momentum family; FAMILY-UNIFORM-FRAGILE.
- **cross-sectional-mean-reversion-strategy** — first pivot family through the harness; FRAGILE.
- **time-series-momentum-robustness** — per-asset absolute momentum (long/flat), the thesis-closing OHLCV test; FRAGILE.
- **horizon-retest-robustness** — coarser 4h/daily decision cadence, the last untested OHLCV axis; FRAGILE.
- **carry-strategy** — cross-sectional funding-carry rotation; FRAGILE.
- **perp-basis-signal-robustness** — perp-spot basis-reversal (long-only), the one LIVE/MEDIUM-HIGH signal the program certified, but fee-gated to fragile in production.
- **perp-basis-mn-spread** — market-neutral basis spread in all three arms; basis ≡ funding byte-identically, residual carries negative median Sharpe — closes the derivatives-positioning channel.
- **simple-strategy-overfit-guard** — overfit/robustness guard proving the down-market trend "hedge" was 2-case noise.
- **simple-strategy-bear-survey** — two-stage bear-market survey over the 2021-22 corpus firming ship-passive on the deepest bear evidence.
- **binance-corpus-expansion** — wider down-market Binance corpus (`data/binance/`, ADR-0032 pins) underpinning the surveys above.
- **passive baseline shipped** — buy-and-hold promoted to canonical production baseline (`spec/runbooks/passive-baseline.md`).

## Retired research lines (negative result; code + anchors retained, not deleted)

- **v2.5 DL forecaster programme** — 4-phase bet (TCN overlay, PatchTST overlay, vanilla Transformer, v2.6 bake-off) plus the TCN alpha-investigation / recalibrate / threshold-tuning / horizon-bump sub-studies: terminal **F4** across two model families, no +0.10 Sharpe-delta on hourly OHLCV. RETIRED 2026-05-22.
- **v3 volatility forecaster** (GARCH-σ position sizing, + noop-fix + re-baseline) — MODEL-BROKEN / NO-ALPHA after the no-op overlay fix. RETIRED 2026-05-22.
- **v3 LLM-forecaster** — reflection-memory + audit-trail-anchored LLM signal; **shipped-partial** (alpha-verdict wave deferred on absent `ANTHROPIC_API_KEY`).
- **v3 regime-classifier / v3 XGBoost cheap-classifier** — OHLCV regime-label predictors; foreclosed when the OHLCV channel was exhausted.
- **vol-killswitch-overlay-noop-fix** — fixed a computed-but-unapplied kill-switch overlay (the precedent behind the day-1 baseline-equity-divergence e2e non-negotiable).

## Cockpit & UI

### Shell, navigation & design system
- **lumen-design-adoption** — master roadmap migrating the cockpit to the Lumen design system + multi-screen sidebar shell (Phase 1 tokens/chrome/status-bar shipped; Phases 2-5 shipped; Phase 6 Assistant slot reserved, gated on v2 LLM).
- **ui-rethink-phase-a-lab** — chart-centric Lab screen.
- **ui-rethink-phase-b-lab-run** — Lab Run button + run plumbing.
- **ui-rethink-phase-c-sidebar-ia** — sidebar IA flip + Live + strategy registry + settings rollup.
- **ui-rethink-phase-d-trail** (+ follow-up) — Trail (audit-journal) view.
- **ui-rethink-phase-e-compare** — Compare matrix.
- **ui-rethink-phase-f-memory-models-assistant** — Memory + Models screens + the Phase-6 Assistant slot.

### Live cockpit & dashboards
- **live-cockpit-unified** — unified single-process cockpit binary.
- **cockpit-live-dashboard-wiring** — equity curve + KPI strip fed from the live agent.
- **live-equity-history-durable** — durable equity history surviving `cockpit_live` restarts.
- **paper-mode-equity-wiring** — real (not flat-line) paper-mode cockpit equity.
- **cockpit-baseline-panel** — surfaces the shipped passive buy-and-hold result.
- **cockpit-reports-viewer** — in-cockpit Reports screen (Library sidebar group): browses the committed `backtest-*.md` corpus and renders the selected report (KPI strip + markdown body + equity curve/drawdown band when the report ships a companion CSV — see `backtest-equity-companion`), reusing the offline viewer's render logic via a shared `crate::reports` loader.
- **backtest-equity-companion** — backtest runs emit a companion `reports/artifacts/<stem>/equity-*.csv` (real per-bar equity) so the Reports screen + offline viewer render a populated equity curve + drawdown band; includes the loader stem-match correctness fix (pair companion→report by file stem, not first-match-any) and 14 committed non-anchored demo reports (real-Binance sma-cross over the 2024 universe). The Reports picker defaults to a **curve-only** filter (with a *show-all* toggle) so switching always lands on a graph — index-safe over the full discovered corpus — and a switch-regression pixel guard proves selecting a different report repaints a distinct curve.
- **cockpit-activity-status-bar** + **-audit-ledger-producer** + **-llm-producer** — continuously-updated "what is the cockpit doing right now" activity bar with audit-ledger and LLM-call producers.
- **cockpit-toast-queue** (+ v0.2 cleanup) — bounded toast queue replacing the single-slot REPLACE semantic.
- **cockpit-training-control** + **cockpit-training-pressed-wiring** — operator-driven `train_tcn` launcher and Train-button wiring.
- **cockpit-performance-and-input-responsiveness** — input-responsiveness + render-performance pass.
- **cockpit-render-regression** — render-regression diagnosis + quality-gate overhaul.

### Charts, tape & journal
- **chart-canvas-overhaul** — chart canvas rewrite.
- **chart-buy-sell-emphasis** — buy/sell marker emphasis from the audit ledger.
- **chart-x-axis-local-time** — local-time x-axis (atomic test-mode override; fixed a `set_var` data race).
- **chart-fixture-line-clipping** — canvas-clip fix (vendored `iced_tiny_skia` fork).
- **tape-row-audit-modal** — click a tape row to open its audit-transaction modal.
- **journal-transactions-metadata** — journal-transactions metadata reader.
- **real-mtm-unrealized-pnl** — real mark-to-market unrealized P&L.
- **per-symbol-position-accounts** — per-symbol position accounts in the ledger.

### Lab (run → save → compare) & data
- **lab-run-save-compare** — real-data strategy checking with durable reports.
- **lab-compare-equity-overlay** — two-run equity overlay on the Compare screen.
- **lab-end-to-end-v2** — closes the Phase A/B gaps, adds a progress bar.
- **lab-polish-round-2** — position curve + param tuning + UI density.
- **lab-recipe-test-harness** (+ v0.2 cross-surface extension) — Lab recipe / subscription test harness.
- **lab-yahoo-realdata** — multi-asset Yahoo data pivot for the Lab (+ v0.1.2 ETH-USD anchor & cache badge, + v0.1.3 REVISION front-matter & Binance ETH H1).
- **lab-yahoo-empty-range-ux** — empty-date-range UX handling.
- **bug-64-d11-attempt-3-yahoo-run-runtime-context** — Yahoo+Run runtime-context + cancellation fix.

### iced platform & UI quality gates
- **iced-native-widgets** + **iced-aw-cherry-pick** + **ui-drop-iced-aw** — migration to native iced widgets, dropping `iced_aw`/`iced_fonts`.
- **ui-gallery-bin** — widget-gallery binary.
- **ui-contrast-asserter** — WCAG contrast asserter (v0.2 enforcing gate, `UI_CONTRAST_MODE`, 6 ratified opt-outs).
- **ui-quality-gate-overhaul** — UI quality-gate overhaul.
- **ui-headless-emulator** + **ui-session-journal-iced-tester** + **ui-test-harness-bootstrap** — headless render adapter, `iced_tester` session-journal adapter, and the panel-snapshot test harness.
- **ui-test-harness-viewport-matrix** — multi-viewport snapshot matrix.
- **visual-fail-html-reporter** — HTML viewer for visual-test failures.

## Core infrastructure

- **reflection-memory** (+ trader-wiring) — persistent lesson-card store with retrieval at decision time, wired through the sanctioned ADR-0041 layering seam.
- **operator-success-reports** — auto-generated "is this working?" reports (equity, Sharpe/Sortino/drawdown, attribution, system health).
- **audit-tick-consumer-envelope** — audit tick consumer with an aggregation envelope.
- **point-in-time-data-discipline** — `core::pit::PitSeries`/`AsOf`: a type-level as-of primitive making look-ahead *unrepresentable* (`AsOf` has private fields + no public ctor; a trybuild compile-fail is the proof), consolidating the hand-rolled as-of joins (funding/basis) behind one guarded API. Behaviour-preserving — anchors 119/119. (ADR-0058, extends ADR-0041.)
- **paper-soak-longevity** (reflection-loop paper-wiring) — wires the reflection writer into the paper trading loop so a lesson card is written on each closed trade (the moat #2 differentiator, previously never wired in the paper path); regime tags are accurate via a BTC-daily-close seed loaded off the async hot path (`spawn_blocking`, no startup hang). Ships the longevity evidence artifact (in-session soak: durable fills, equity movement, restart-continuity, kill-switch, lesson accumulation) + an operator runbook for the real-time 90-day soak. Regression guard: `reflection_wiring_regression.rs` (with a no-writer negative control).

## Tooling & process

- **operator-ledger-schema-lint** — ledger chart-of-accounts schema lint.
- **adr-registry-atomic-lint** — ADR-registry atomicity lint (sibling pre-commit guard).
- **queue-staleness-reconciliation** — backlog-queue staleness reconciliation pass.
- **subscription-pipe-server-time-template** — server-time template closing the Wave-1 subscription-pipe carve-out.
- **regression-anchor gate** — `spec/anchors.toml` + `scripts/verify_anchors.sh` (119 byte-SHA bodies); `scripts/spec_lint.py` structural gate (dead-link, frontmatter, orphan, trace, status-drift).

## Deferred / not built (by decision)

- **cockpit-cross-platform** — Linux/Windows source shipped + macOS-verified; 3-OS CI matrix parked inert (`.github/workflows/ci.yml.deferred`), activation deferred to the near-done milestone.
- **cockpit-app-bundle**, **iced-ecosystem-evaluation**, **ui-gallery-table-cell** — candidate/draft, not built.
- **Out of scope (follow-up project):** real-money execution, KYC, exchange API keys, withdrawals, multi-venue real-money, tax/lot accounting.
