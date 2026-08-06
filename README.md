# Trading

A **Rust crypto trading agent** — now shipped as **"The Honest Advisor,"** a single-coin paper/sim investment advisor (pick a coin + budget → bake off all strategies → rank under a robustness gate → forward paper-trade your €200) — built on a spec-driven research engine with persistent reflection memory and a double-entry audit ledger. Operates on real-data backtesting + paper simulation over crypto pairs; **paper/sim only** (live trading is out of scope), not financial advice.

This README is the human entry point. AI agents should start at **[CLAUDE.md](CLAUDE.md)** then **[AGENT.md](AGENT.md)**.

---

## Status — FEATURE-COMPLETE (2026-07-09) · UNDER REVIEW HARDENING (since 2026-07-26)

**The product is "The Honest Advisor" — a single-coin paper/sim investment advisor.
Its FEATURE LIST is complete; its PRODUCT STATE is mid-hardening.** Both halves are
true and the second one is easy to lose: a code-review burn-down over the frozen
`review`-status stories (7 of 14 closed) has disclosed five defects of the
"declared ≠ executed" family (bug-log #65-#69) — including cross-symbol fill
mispricing that makes several anchored RESEARCH surfaces execution-artifact noise
(#67) — and opened two CRITICAL re-lock stories (1-24, 1-25) plus three scoped
builds. **What that does NOT touch:** the advisor's bakeoff gate resamples returns
and never re-executes fills (verified three times adversarially), so crowns,
verdicts and the era-qualified ship-passive conclusion stand independently. The
active-trading-thesis closure currently reads *direction-preserved pending re-lock*.
Current honest state: `_bmad-output/implementation-artifacts/sprint-status.yaml`,
`docs/dev-notes/bug-log.md`, `docs/dev-notes/product-review-2026-08-04.md`.

The honest arc that got here:

1. **The active-vs-passive research program CONCLUDED (2026-06-08): SHIP PASSIVE (current era).** Across
   all three reachable channels — price/OHLCV, derivatives-positioning, and on-chain — no
   active strategy beat passive buy-and-hold net of cost under a frozen, pre-registered
   block-bootstrap robustness rule, on the current deep-liquidity market (firmed on real
   2021-22 bear-market data). **This result is the moat, not a disappointment** — it is kept
   prominent because it is the product's credibility. And the *scope* of the claim is itself
   measured: the P2 corpus-expansion verdict re-run
   ([`evidence/v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun.md`](evidence/v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun.md))
   ran the same gate back across the older, thinner-liquidity eras and found **real,
   cost-annex-robust active edges in the early market (2017-20) that decay to ~zero by 2023+** (gate-crowned; post scorecard-fix none is DSR-certified — see the [errata](evidence/v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun-errata.md)) —
   the efficiency-migration pattern the research predicted. The gate did not merely fail to
   find alpha; it *positively detected* real historical edges and the boundary where they died
   (qualified by survivor-of-survivors bias + old-era cost realism — those margins are upper
   bounds). Forward advice is unchanged: every window the advisor runs ends at "now", where
   holding still wins.
2. **Product pivot (2026-06-19): "The Honest Advisor."** The shipped engine was re-framed
   into a guided retail journey — *pick a coin + budget (e.g. €200 XRPUSDT) → bake off ALL
   strategies → rank the best under the robustness gate → forward plan → paper-trade your
   €200 and watch the P/L.* The ship-passive verdict became a **feature** of it: buy-and-hold
   is the always-present benchmark arm, and when nothing active clears the bar (the modal
   real-crypto outcome) the advisor honestly crowns "just hold" (`BenchmarkWins`).
3. **v2 (11 research-driven features) + v3 close-out shipped.** The 900-paper research
   program's entire ship-worthy tranche landed as v2 (overfitting scorecard, turnover/tail
   metrics, confidence-not-verdict, forward-coverage, vol-estimator/overlay, drawdown-overlay,
   opt-in cost model, narration-faithfulness, no-alpha CI, data-quality surface — ADRs
   0075–0081). The v3 **"prove it's done"** close-out then added the Calibrate-stage stepper
   (ADR-0083), the do-not-build register, the DSR report-only decision, and the end-to-end
   demo runbook.

The thesis has now been stress-tested from every reachable angle — long, combinations,
shorts, breakout/volume/OBV signals, implied-vol regime, macro cross-asset, and (P2) four
extra market eras + a second venue — and on the current deep-liquidity market it **holds
every time**. The one place it *bends* is the honest one: on the older, less-efficient eras
(2017-20) the same gate crowned real, cost-annex-robust active edges that have since decayed (gate-crowned, NOT DSR-certified after the scorecard variance fix) —
which is the gate **working**, positively mapping the boundary of its own claim rather than
asserting a universal that isn't true (this is a *strength* of measured honesty, not a crack;
those old edges are unreachable today and are the exact alpha-chasing the do-not-build register
forbids). There is no coherent "add-more-features" v3 (see
[`docs/dev-notes/post-v2-scoping-2026-07-09.md`](docs/dev-notes/post-v2-scoping-2026-07-09.md));
manufacturing more alpha surface would contradict *measured honesty, not asserted alpha*.

| Dimension | State |
|---|---|
| Mode | **Feature-complete single-coin paper/sim advisor ("The Honest Advisor").** Research CONCLUDED — **ship passive** (the credibility layer). Real-data backtest + paper sim. Live trading removed from scope 2026-06-12 (not wired, not planned). |
| Workspace | Rust stable, edition 2024 |
| Test gates | **119/119 anchored body-SHAs byte-identical**; full lib/integration/UI-snapshot suite green. Visual-regression gate de-flaked (a multithread `set_var` race was randomizing chart renders); WCAG contrast gate ENFORCING. |
| CI | 3-OS (Linux/Windows/macOS) cross-platform source shipped + macOS-verified; the GitHub Actions matrix stays **operator-parked** inert at `.github/workflows/ci.yml.deferred` — the "near-done" milestone is reached but the operator kept it parked in the v3 close-out (do not activate without operator direction). |
| UI | Cockpit shipped (`cockpit_live` + `cockpit` fixtures binaries, iced 0.14.0 + vendored `iced_tiny_skia` patch). The advisor journey re-centres it on **DATA → CALIBRATE → ANALYZE → SUGGEST** with a visible stepper band (ADR-0083). |
| Advisor product | Pick coin + budget → bake off ALL strategies → rank under the robustness gate → forward plan → forward paper-trade the €200. MVP (F1–F9 + EUR-FX + dynamic data) + v2 tranche + v3 close-out all shipped. Honest `BenchmarkWins` when nothing active clears the bar. An end-to-end demo runbook exists (awaiting operator approval). |
| Strategies | SMA, composed (MACD/RSI/Bollinger), cross-sectional momentum, mean-reversion pairs, multi-venue, vote-ensembles, signal-library (Donchian/volume-breakout/ROC/OBV), directional shorts, DVOL/macro exogenous arms, LLM-as-narrator — all shipped, all judged by the same frozen gate + buy-and-hold benchmark; none robustly beats holding. |
| Strategy research retired | The full DL chain (TCN/PatchTST/GARCH/Transformer), LLM-/xgboost-/regime-forecasters, AND the derivatives perp-basis market-neutral spread (FAMILY-UNIFORM-FRAGILE) + on-chain (PIT-infeasible) — i.e. the entire active-edge search. |
| Data | 10-symbol Binance hourly 2023-24 (pinned `3a8b96c4`) + a 2021-22 bear corpus (pinned `4f390622`) + on-demand dynamic fetch for any coin/window + Deribit DVOL and Yahoo cross-asset/macro corpora; the fetcher is idempotent for gapped months. |

For the feature-by-feature index see [`CHANGELOG.md`](CHANGELOG.md) (one line per implemented
feature, grouped by subsystem/version). The settled dead-ends that should NOT be re-proposed
are consolidated in the [**do-not-build register**](docs/dev-notes/do-not-build-register.md);
the whole spine hanging together on one golden input is walked in the
[**end-to-end demo runbook**](docs/runbooks/advisor-end-to-end-demo.md).

---

## What this project does

**The product — "The Honest Advisor" (2026-06-19 pivot).** A single-operator, paper/sim
decision-support tool that answers one concrete question: *"I have €200 for one crypto
(say XRPUSDT) — which strategy should I use, and what should I do over the next few days?"*
The guided journey: **pick** a coin + budget → **bake off** every available strategy over a
configurable 2-week-to-4-year window → **rank & select** the best by risk-adjusted Sharpe
under a Monte-Carlo robustness gate (with a plain-language "why this one") → **plan** a
budget-aware, rule-driven forward stance → **watch** the selection paper-trade your simulated
€200 forward on real data. It is **paper/sim only** (no live orders), **not financial advice**,
and **single-coin** by design. Its credibility comes from the concluded research verdict, not
from an alpha claim: **buy-and-hold is always in the bake-off as the benchmark**, and when no
active strategy robustly beats it — the modal real-crypto outcome — the advisor says so plainly
(`BenchmarkWins`). Full spec: [`_bmad-output/planning-artifacts/PRD.md`](_bmad-output/planning-artifacts/PRD.md).

**Built on a shipped research engine (the moat, not waste).** The advisor is a re-framing of
an existing, working stack with two durable differentiators that remain its trust layer:
1. **Persistent reflection memory** (`crates/reflection`): every shipped strategy decision + outcome is stored as a `LessonCard` with a 32-dim deterministic embedding, retrievable by symbol/regime via `retrieve_top_k`.
2. **Auditable double-entry ledger** (`crates/audit`): every fill, fee, slippage, LLM call, and strategy emit is recorded as journal transactions with full body-SHA-256 anchoring for byte-identical regression gates.

Alongside these sit the backtest/matching engine, the strategy library, the Monte-Carlo
robustness harness (the credibility layer that gates every pick), the LLM integration (narration
only — never the alpha source), and the paper simulator — all reused by the advisor journey.

**Cockpit.** A native iced app (`cockpit_live`) surfaces strategy state, equity curves, drawdowns, positions, audit trail, reflection memory, and an Assistant slot for LLM reasoning traces. The advisor journey re-centres it on a visible **DATA → CALIBRATE → ANALYZE → SUGGEST** stepper (ADR-0083) over the existing sidebar IA.

**Story-driven workflow (BMAD-METHOD v6, migrated 2026-07-25).** Every feature is a story under `_bmad-output/implementation-artifacts/` (one file per feature, grouped into 7 epics), planned from `_bmad-output/planning-artifacts/` (PRD, architecture spine + ADRs, epics, trace ledger) and evidenced by the byte-immutable corpus under `evidence/`. The workflow cycle (sprint-status → create-story → dev-story → code-review → retrospective, with customized persona agents) is documented in [AGENT.md](AGENT.md).

---

## Quickstart

### Build

```bash
# Workspace build (debug)
cargo build

# Cockpit (release; needs `live` feature)
cargo build --release -p ui --bin cockpit_live --features live

# Backtest binary (release)
cargo build --release -p backtest --features realdata,candle
```

### Configure

```bash
# 1. Set up local secrets file (git-ignored)
cp config/agent.toml.local.example config/agent.toml.local

# 2. Edit config/agent.toml.local with your real Anthropic/OpenAI/etc keys
#    (only needed if running LLM-strategy code paths)

# 3. Toggle config/agent.toml [llm] enabled = true to activate LLM subsystem
```

The committed `config/agent.toml` carries the shape; `config/agent.toml.local` overlays secrets at startup. See `config/agent.toml.local.example` for the template.

### Run the cockpit

```bash
# Canonical interactive run — ALWAYS use a release build.
cargo run -p ui --release --bin cockpit_live --features live
# (or run the prebuilt binary)
./target/release/cockpit_live
```

> **Run the cockpit in release.** It renders through the CPU `tiny-skia`
> rasterizer (chosen for snapshot-test determinism over GPU `wgpu`). At the
> dev default `opt-level = 0` a single Lab/Charts frame takes **~700 ms** to
> rasterize vs **~17 ms** in release — a measured **40× debug-tax** that
> shows up as the "1–3 s per interaction" lag. The workspace
> `[profile.dev.package."*"]` override (root `Cargo.toml`) now compiles the
> rasterization *dependencies* at `opt-level = 3` even in dev, so a plain
> `cargo run … --features fixtures` is usable too — but `--release` remains
> the canonical, fastest path. Numbers: `crates/ui/tests/render_timing_probe.rs`.

Opens the iced window. Default screen is **Lab** (strategy experimentation). Other screens are reachable via the left sidebar: Live, Compare, Memory, Models, Trail, Strategies, Risk, Audit, Control, Settings, Charts, Home, Debug.

### Run a backtest

```bash
# Synthetic data (always works)
cargo run -p backtest --release -- --scenario top10-2023-fy-momentum --seed 0xC0FFEE

# Real Binance data (requires data/binance/ tree populated + --features realdata)
cargo run -p backtest --release --features realdata,candle --bin backtest -- \
  --scenario top10-2023-fy-momentum-realdata --seed 0xC0FFEE
```

Backtest reports land under `evidence/<feature>/reports/backtest-<stamp>-<scenario>.md` with body-SHA-256 anchored for regression.

### Verify regression gates

```bash
# All shipped backtest body-SHA-256 anchors
bash scripts/verify_anchors.sh

# Spec structural integrity (dead links, missing frontmatter, etc.)
uv run scripts/spec_lint.py

# Full Rust validation pipeline
cargo fmt --check
cargo clippy --workspace --features candle,realdata -- -D warnings
cargo test --workspace --lib --features candle
```

---

## Features at a glance (grouped)

Full per-feature index: see [`CHANGELOG.md`](CHANGELOG.md) — one line per implemented feature, grouped by subsystem/version.

> The tables below sample the **engine-era** strategy/research work these are built on. The
> current product surface — **"The Honest Advisor"** (bake-off + ranking + forward plan +
> paper-trade, the MVP + v2 tranche + v3 close-out) — is indexed in the CHANGELOG's
> [advisor section](CHANGELOG.md#single-coin-investment-advisor-paper--2026-06-19-pivot-mvp-shipped).

### Trading strategies (shipped)

| Feature | Version | Purpose |
|---|---|---|
| `v0-paper-sma` | 0.1.0 | SMA crossover paper baseline |
| `v05-composed-strategies` | 0.5.0 | `with_*` composition framework |
| `v1-cross-sectional-momentum` | 1.0.0 | Production momentum on top-10 universe; the load-bearing baseline |
| `v15a-mean-reversion-pairs` | 1.1.0 | Pairs trading framework |
| `v1-5b-multi-venue` | 1.2.0 | Multi-venue execution support |
| `v2-llm-strategy` | 2.0.0 | LLM-as-analyst infrastructure (RecordingProvider/ReplayProvider/BudgetedProvider/CachedSystemPromptBuilder) |
| `v3-llm-forecaster` | 0.1.0-PARTIAL | LLM-as-forecaster with reasoning trace; Wave D deferred pending API key |

### Strategy research (retired chains)

| Chain | Outcome |
|---|---|
| `v25-dl-forecast-overlay` umbrella + TCN + PatchTST | Joint F4-F4-F4 verdict; retired 2026-05-22 |
| `v3-volatility-forecaster` + rebaseline | MODEL-BROKEN / NO-ALPHA / NEGATIVE-NET-DELTA; retired 2026-05-22 after noop-fix |
| `v3-regime-classifier` | Draft only; never promoted |

Retired code stays in the tree (no deletion); anchors stay locked. See [`docs/dev-notes/retired-surface-inventory-2026-05-22.md`](docs/dev-notes/retired-surface-inventory-2026-05-22.md) for the inventory.

### Infrastructure (shipped, load-bearing)

- `crates/audit/` — double-entry ledger + journal transactions + per-symbol position accounts + audit-tick stream
- `crates/reflection/` — LessonCard store + 32-dim embeddings + `top_k` retrieval + 3-state regime tagger
- `crates/llm/` — `LlmProvider` trait + Anthropic/OpenAI-compat/Ollama providers + RecordingProvider/ReplayProvider for determinism
- `crates/backtest/` — `MatchingEngine` + scenarios + report rendering with body-SHA-256 anchors
- `crates/strategy/` — `Strategy` trait + `Strategy::quantity_scale` sizing hook + all strategy implementations
- `crates/ui/` — iced cockpit (14 screens; Lab/Live/Compare/Memory/Models/Trail/Strategies/Risk/Audit/Control/Settings/Charts/Home/Debug)
- `crates/exec/` — fill publisher shim (note: real matching engine still lives in `crates/backtest/`; architect-flagged for rename per `docs/dev-notes/feature-state-architect-review-2026-05-22.md`)

### UI surface

Native iced cockpit with operator-facing screens for strategy state, equity curves, drawdowns, positions, audit trail, reflection memory, and an Assistant slot for LLM reasoning. Kill switch (typed-confirm modal) is the load-bearing destructive control. Risk override + risk veto flows shipped Phase 5.

---

## Project structure

```
trading/
├── README.md           # This file
├── CLAUDE.md           # AI agent entry point — workflow + non-negotiables
├── AGENT.md            # Multi-agent orchestration contract
├── Cargo.toml          # Workspace root (16 crates)
├── config/
│   ├── agent.toml          # Committed config (shape only; no secrets)
│   ├── agent.toml.local    # Git-ignored secrets overlay (you create this)
│   └── strategies/         # Per-strategy TOML configs
├── crates/             # Rust workspace (audit, backtest, core, cost, data,
│                       #   exec, features, forecast, llm, models, reflection,
│                       #   reports, risk, strategy, ui)
├── data/
│   ├── binance/        # Parquet OHLCV (gitignored; populate manually)
│   ├── audit/          # SQLite audit ledger
│   ├── reflection/     # SQLite reflection store
│   └── llm-replay.db   # Deterministic LLM response cache
├── scripts/
│   ├── verify_anchors.sh   # Regression gate: 119 body-SHA-256 anchors
│   ├── spec_lint.py        # Structural integrity (stories/board/trace/CHANGELOG triad)
│   ├── hash_report.py      # Canonical body-SHA hasher
│   └── check_presentation.sh
├── evidence/           # Byte-immutable corpus (moved out of spec/ 2026-07-25)
│   ├── anchors.toml        # 119 locked body-SHA-256 regression anchors
│   └── {v1,v2,v3,…}/<feature-slug>/
│       ├── reports/        # Anchored backtest + test reports (frozen)
│       └── presentations/  # Operator-approval decks (frozen)
├── docs/               # Project knowledge
│   ├── dev-notes/          # Cross-cutting memos + audits + do-not-build register
│   ├── runbooks/           # Operational runbooks
│   ├── design/             # Lumen design system (+ ui-design-principles.md)
│   └── archive/            # Frozen history: pre-bmad-spec/ (retired spec/ tree),
│                           #   pre-bmad-agents/ (retired .claude/agents/)
├── _bmad/              # BMAD-METHOD install (config, manifests) + custom/ overrides
├── _bmad-output/
│   ├── planning-artifacts/     # PRD.md · architecture.md (AD-1..AD-19 spine)
│   │                           #   · architecture/decisions/ (ADRs) · epics.md
│   │                           #   · trace.toml · backlog.md
│   └── implementation-artifacts/   # sprint-status.yaml + one story per feature
├── target/             # Cargo build output
└── vendor/
    └── iced_tiny_skia/ # Long-term local fork (operator-locked per CLAUDE.md)
```

### Top-level files for AI agents

| File | Purpose |
|---|---|
| **[CHANGELOG.md](CHANGELOG.md)** | Canonical "what's been built" index — one line per implemented feature, by subsystem/version; the third leg of the ADR-0082 triad. Full narrative in `git log`. |
| **[CLAUDE.md](CLAUDE.md)** | Project rules + non-negotiables + coding conventions. AI agents read this first. |
| **[AGENT.md](AGENT.md)** | BMAD orchestration contract: persona mapping, the workflow cycle, orchestrator duties, capability boundaries. |
| **[_bmad-output/planning-artifacts/PRD.md](_bmad-output/planning-artifacts/PRD.md)** | What this project is and isn't. |
| **[_bmad-output/planning-artifacts/architecture.md](_bmad-output/planning-artifacts/architecture.md)** | The architecture spine — 19 binding invariants (AD-1..AD-19); ADRs under [`architecture/decisions/`](_bmad-output/planning-artifacts/architecture/decisions/README.md) authoritative on conflict. |
| **[_bmad-output/planning-artifacts/epics.md](_bmad-output/planning-artifacts/epics.md)** | The 7 epics grouping every story (shipped tranches + the open tail). |
| **[_bmad-output/implementation-artifacts/sprint-status.yaml](_bmad-output/implementation-artifacts/sprint-status.yaml)** | The live board — epic/story statuses; maintenance posture. |
| **[_bmad-output/planning-artifacts/backlog.md](_bmad-output/planning-artifacts/backlog.md)** | Forward-looking Queue only (shipped work lives in CHANGELOG.md). |

---

## Conventions

### Story status vocabulary (per `scripts/spec_lint.py` `VALID_STORY_STATUSES`)

Each feature is a story file with a `Status:` line — the lifecycle source of
truth since the 2026-07-25 BMAD migration (story-keyed AD-4):

| Status | Meaning |
|---|---|
| `backlog` | Exists in the epic file / board only; no committed work |
| `ready-for-dev` | Story file created with full context; awaiting implementation |
| `in-progress` | Developer actively working |
| `review` | Awaiting/under code review (also the frozen home of pre-migration `presenter/tester/dev-done` states) |
| `done` | Shipped: code on main, anchors locked, trace row `shipped`/`shipped-partial`, CHANGELOG line present |
| `retired` | Research line closed; code stays in tree; anchors locked; no further effort |

The richer pre-migration vocabulary (`draft`/`proposed`/`shipped-partial`/…)
lives on in `trace.toml` `state=` values; `scripts/spec_lint.py` maps and
cross-checks the two (`status-drift`).

### Anchored body-SHA-256 regression gates

Every shipped backtest report has a body-SHA-256 entry in `evidence/anchors.toml`. The gate is `bash scripts/verify_anchors.sh` (must report `ANCHORS PASS (N / N)` before any ship). Bodies are byte-immutable; documentation-link cleanup sweeps MUST exclude anchored files (see CLAUDE.md non-negotiables).

### BMAD workflow

Per [AGENT.md](AGENT.md): the BMAD v6 cycle — `bmad-sprint-status` →
`bmad-create-story` → `bmad-dev-story` → `bmad-code-review` → story `done`
(triad move) → `bmad-retrospective` per epic — with the customized persona
agents (`_bmad/custom/` overrides) and the project harness skills
(`rust-*`, `verify-anchors`, `spec-lint`, `backtest`, `cockpit-smoke`).
Every non-trivial change runs through this loop; the orchestrator runs
sub-agents in parallel when independent and owns all commits. Trivial
one-file edits skip the loop.

### Durable writes

Durable output lands in `_bmad-output/` (stories, board, planning docs),
`docs/` (knowledge), or `evidence/` (reports — byte-immutable once anchored)
via the BMAD workflows' write-paths. The legacy `spec-update` skill is
retired (ratified decision D5).

---

## Non-negotiables (from CLAUDE.md — the full enumerated list lives there)

- No secrets in git. Keys in `config/agent.toml.local` (git-ignored) or env vars per [`_bmad-output/planning-artifacts/architecture.md`](_bmad-output/planning-artifacts/architecture.md).
- No shipping on a `REGRESSION` verdict without an explicit human override.
- No silent divergence from [`_bmad-output/planning-artifacts/architecture.md`](_bmad-output/planning-artifacts/architecture.md) (the AD-1..AD-19 spine; the FROZEN robustness gate is byte-frozen).
- **Every strategy overlay or sizing-modifier ships with a baseline-equity-divergence e2e test from day 1** (precedent: `v3-volatility-forecaster-noop-fix` 2026-05-22; the noop bug went undetected by 5 sequential gates).
- **Anchored report files in `evidence/*/reports/` are byte-immutable** per ADR-0038 § D6; even mechanical link-fix edits mutate the body-SHA. Documentation-link cleanup MUST exclude anchored files. Anchors are 119/119 before AND after any change, keyed by scenario NAME.
- **The do-not-build register is binding; the thesis is era-qualified** — see [`docs/dev-notes/do-not-build-register.md`](docs/dev-notes/do-not-build-register.md); never state the universal no-active-edge form.

---

## Key dev-notes

When orienting on the project, these dev-notes give the most signal per word:

- [`CHANGELOG.md`](CHANGELOG.md) — the comprehensive per-feature inventory (supersedes the retired `feature-state-table` snapshot)
- [`v25-dl-journey-retrospective-2026-05-22.md`](docs/dev-notes/archive/2026-Q2/v25-dl-journey-retrospective-2026-05-22.md) — what the forecaster track taught (4 retirements)
- [`v3-vol-overlay-noop-discovery-2026-05-22.md`](docs/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md) — the load-bearing engineering pattern
- [`strategic-reset-2026-05-23.md`](docs/dev-notes/archive/2026-Q2/strategic-reset-2026-05-23.md) — half-validated moat finding + next-6-week roadmap framing
- [`feature-state-analyst-review-2026-05-22.md`](docs/dev-notes/archive/2026-Q2/feature-state-analyst-review-2026-05-22.md) + [`feature-state-architect-review-2026-05-22.md`](docs/dev-notes/archive/2026-Q2/feature-state-architect-review-2026-05-22.md) — dual-perspective reviews

---

## License

Private / single-operator research project. No external license at this time.
