---
title: "PRD Addendum — The Honest Advisor (engine reference & decision rationale)"
status: final
created: 2026-07-24
updated: 2026-07-24
---

# PRD Addendum — The Honest Advisor

> Companion to `PRD.md`. Migrated from `spec/product.md` 2026-07-24 (BMAD Phase 1); `spec/` remains authoritative until Phase 5b cutover. This addendum preserves depth that belongs downstream of the PRD — the engine-reuse inventory, engine mechanics retained for the architect, and the options-considered rationale behind decisions D1–D5.

## A. The engine being reused (inventory — prior work, not waste)

Everything below was already shipped before the 2026-06-19 pivot and became the engine of the guided journey:

| Capability | Crate / screen | Role in the journey |
|---|---|---|
| Backtest engine (matching, fills, friction) | `crates/backtest` (`MatchingEngine`, `run_scenario`) | Runs each arm on `(coin, window)` |
| Risk-metric stats | `crates/backtest/src/stats` (`compute_sharpe_*`, `compute_sortino_*`, `compute_calmar`) + `BacktestKpis` (`total_return_pct`, `max_drawdown`, `trade_count`) | The ranking inputs — no new math needed |
| Strategy library | `crates/strategy` (SMA, composed MACD/RSI/Bollinger, cross-sectional momentum, mean-reversion pairs, regime dispatcher, LLM/ML overlays) | The bake-off field |
| Passive baseline | shipped buy-and-hold control (`runbooks/passive-baseline.md`) | The benchmark arm + safe default |
| Robustness harness | `monte-carlo-bootstrap-path-generator` + `strategy-robustness-harness` | The credibility / fragility flag |
| Single-run backtest UX | cockpit **Lab** screen (`crates/ui/src/lab`, `runner::spawn_lab_run`) | The bake-off is essentially looping Lab over the registry |
| Forward paper-sim | `crates/agent` runtime (paper mode) + EventBus | Drives the forward paper-trade |
| Live P/L view | cockpit **Live** screen (`crates/ui/src/live.rs`) | Watch the selection paper-trade the €200 |
| LLM integration | `crates/llm` (Anthropic / OpenAI-compat / Ollama, recording/replay) | The "why this one" narration (+ the formerly-planned analyst overlay) |
| Reflection memory | `crates/reflection` (LessonCards, retrieval) | Optional: surface relevant past lessons |
| Audit ledger | `crates/audit` (double-entry, body-SHA anchors) | Makes every paper fill + recommendation reproducible |
| Reports + viewer | `crates/reports`, cockpit Reports screen | Renders the bake-off / forward-run evidence |

The net-new surfaces of the pivot were: the bake-off orchestrator, the ranking/recommendation surface, the forward-plan generator, budget-aware sizing, and the guided "new investment" input — all since shipped (PRD §8.1).

## B. Engine reference (mechanics retained for the architect)

Descriptive of shipped capability, not scope. The pivot composed these; it did not change them.

### B.1 Strategy library (the bake-off field)

The `crates/strategy` registry holds named strategies sharing data/feature/risk/exec scaffolding: SMA crossover, composed multi-indicator (MACD + RSI + Bollinger), cross-sectional momentum, mean-reversion pairs, the regime dispatcher, and the v2 LLM-as-analyst overlay. The retired forecaster chains (TCN / PatchTST / GARCH-σ / LLM-forecaster) remain in the tree behind feature flags; the bake-off includes a strategy only when it runs cleanly on a single `(coin, window)` — the retired ML overlays are **opt-in**, not default arms, given their concluded negative verdicts. Post-pivot arm classes (vote ensembles, the short slate, the signal-library expansion, the DVOL and macro exogenous probes) are pre-registered slates per PRD FR-18.

**Active-edge-search status (2026-06-08, retained).** Across the three reachable channels (price/OHLCV, derivatives-positioning, on-chain) no active strategy beat passive buy-and-hold net of cost under the frozen block-bootstrap Monte-Carlo rule (passive: +1.74 Sharpe 2023 / +1.10 2024). A **bounded** result on the 2023-24 large-cap sample — not a claim active trading is impossible; untested channels remained by lower prior or infeasibility (the DVOL and macro probes have since covered two of them, both null). The methodological spine (frozen pre-registered rule, block-bootstrap MC, byte-SHA anchors, day-1 falsifiers, anti-cherry-pick renderer, live-bar calibration) is preserved in git history and `runbooks/passive-baseline.md`. The era-qualification of this verdict (2026-07-10, P2 corpus expansion) is load-bearing and lives in PRD §2.

### B.2 Robustness machine (the credibility layer)

The Monte-Carlo robustness layer resamples *real* returns (stationary block bootstrap, Politis–White auto block length) into an ensemble of plausible paths and measures the **distribution** of a strategy's outcome (Sharpe p5/p50/p95, max-drawdown tail, probability of loss) against a pre-registered rule (p5 Sharpe < 0 → FRAGILE). This is **uncertainty quantification, not prediction**. It powers the fragility flag on the leaderboard (PRD FR-6) and the gate-tied tuning verdicts (FR-16).

### B.3 LLM role (support — never the alpha source)

Empirical basis: three retired alpha-by-prediction bets (TCN/PatchTST, GARCH-σ, LLM-forecaster). Sanctioned roles: the "why this one" narration (FR-11); lesson summarization / narrated tie-breaks (auditable, never the primary ranking gate). Runtime mechanics: dual-tier (`deep_think` / `quick_think`), provider abstraction (Anthropic default with prompt caching; OpenAI-compatible; local Ollama), hard monthly token budget with 80%/100% auto-degrade, tool-use schemas over free-text parsing, record/replay for tests. Note: the once-aspirational "LLM-as-analyst bake-off arm" was ratified against code as **not built** — LLM/ML are narration-only (PRD D3), and the LLM crate is imported by neither strategy nor backtest.

### B.4 Data sources

Real market data only: spot OHLCV from **Binance** (pinned hourly 2023-24 corpus + a 2021-22 bear corpus; P2 added 2017-18, 2020, 2025-26 and a Coinbase second-venue corpus) and multi-asset **Yahoo** data for the Lab (with a market-calendar layer so equity/FX/rates tickers pass coverage checks). Exogenous series: Deribit DVOL (implied vol) and the Yahoo macro set — both joined through the as-of/PIT primitive. Funding/open-interest and on-chain feeds exist from the research program but are not required for the single-coin journey. On-demand fetch covers `(coin, window)` outside the pinned corpora (git-ignored cache, anchor-safe).

### B.5 Risk management (hard requirements, retained)

Risk limits enforced as Rust types — illegal orders fail at construction; the budget cap is itself a hard limit (paper sizing may never deploy more than the simulated budget); kill switch (halt file / missed heartbeat → flatten + stop); per-symbol exposure cap; max-drawdown trigger; full audit log. Sizing is fixed-fraction with optional de-risk-only overlays (vol-targeting, drawdown-control) — "size down, control risk," never "size up for alpha."

### B.6 Operating modes (paper is terminal)

1. **Research** — backtest only, deterministic seeds, cached LLM replay. The bake-off runs here.
2. **Paper** — live data feed, full pipeline, simulated fills, real LLM cost. The forward paper-trade runs here.
3. ~~Live~~ — **removed from scope 2026-06-12.** Not wired, not planned.

### B.7 Cockpit information architecture

One-screen operator cockpit; Lumen design system; sidebar shell with Home / Charts / Strategies / Risk / Audit / Debug / Lab / Live / Compare / Memory / Models / Trail / Reports screens. The pivot re-centred the journey on **Lab → Bake-off/Leaderboard → Plan → Live** (plus the Calibrate stage and the DATA → CALIBRATE → ANALYZE → SUGGEST stepper band) without redesigning the shell. Order entry / config editing / multi-account remain out of the cockpit IA (paper tool, single operator, config-driven universe).

### B.8 Strategy lifecycle — promotion gates

A strategy lives in one stage at a time; promotion is explicit and criteria-driven (`research` → `paper` requires a single-path OOS Sharpe > 1.0 **and** a robustness distribution read against the pre-registered rule). The bake-off + recommendation is a user-facing instance of the `research` gate, packaged for a non-expert.

### B.9 Cost economics

Monthly opex ladder ($45 / $135 / $360) with the 80%/100% LLM auto-degrade rule. The single-coin journey is cheap in research mode (no LLM); LLM cost appears only in the opt-in narration (one cheap call per recommendation, cacheable).

### B.10 Operator success reports

Auto-generated "is this working?" reports (equity, Sharpe/Sortino/Calmar/drawdown, attribution, system health). The bake-off result is itself a report — a ranked, dated, reproducible artifact the user can re-open.

## C. Key product decisions — options considered (D1–D5 rationale)

**D1 — ranking metric.** Chosen: **(a) risk-adjusted with a robustness gate** (durable: the metric the codebase's robustness thesis is built on; protects a naive user from a lucky-path leaderboard). Rejected: (b) total return only — rewards a single lucky path and a 73%-drawdown ride equally with a smooth one; actively misleads a small-budget user. (c) a blend score — introduces a magic-weights decision to defend; deferred until evidence (a) is insufficient (none arose).

**D2 — the forward "plan."** Chosen: **(a) current stance + rules + a forward paper-run** — you cannot pre-compute future orders for a price-dependent strategy (an SMA cross depends on prices that haven't happened); the Live view IS the plan unfolding. Rejected: (b) a deterministic order schedule — only correct for the rare price-independent strategy (e.g. fixed DCA); misleading as the general plan.

**D3 — ensemble / LLM-mix scope.** Chosen: **(a) later (v0.2)** — MVP ranked one strategy; ensembles shipped afterwards as bounded, pre-registered signal-vote mixes earning the crown through the same gate + benchmark. Rejected: (b) in the MVP — would have tripled the surface area (mix sizing, ensemble arbitration, LLM cost in the hot loop) before the basic loop was trustworthy. LLM/ML remained narration-only throughout.

**D4 — EUR budget on a USDT-quoted pair.** Chosen: **(a) for MVP** (budget as quote-units, honestly labelled "FX not modelled") then **(b) as the shipped refinement** (one-time conversion at a configurable static rate, honest "€200 ≈ $X (at R EUR/USD)" display; ranking FX-invariant). Rejected: (c) first-class `Eur` currency + live FX feed — a new currency impl, an FX data source, and FX-PnL plumbing through the ledger; out of scope for a paper decision-support tool. The rate-source fork (static default vs live-fetched v0.3) is PRD §13.1; a corpus-derived rate was rejected (no corpus FX series).

**D5 — paper-only.** **Re-affirmed** (standing constraint since live execution was removed 2026-06-12): the €200 is simulated; not-advice + simulated-budget disclaimers on every recommendation/Live surface. No alternative was on the table.

## D. Provenance & non-migrated material

- The **product changelog** embedded in `spec/product.md` (2026-04-17 → 2026-07-10, including the pivot record, per-feature scoping entries, and the terminal ship-passive verdict entries) is deliberately **not** migrated into the PRD: git history remains the narrative record, and `CHANGELOG.md` remains the shipped-feature index (and stays at the repo root through the migration).
- The **do-not-build register** (`docs/dev-notes/do-not-build-register.md`) remains the authoritative dead-end reference; PRD §7.2 summarizes it and defers to it. It moves to `docs/` in migration Phase 4 without content change.
- The **era-qualified thesis wording** in PRD §2.1 is carried verbatim in meaning from the register's Group-A preamble (the post-P2, operator-ratified phrasing). Any future edit to that wording happens at the source first.
- Engine-shape facts asserted here (USDT denomination, ranking inputs, the bake-off as a loop over the Lab runner, the forward run extending the paper runtime) were code-verified during the 2026-06-19 pivot analysis and re-confirmed by the shipped features they scoped.
