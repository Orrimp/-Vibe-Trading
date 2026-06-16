# Trading

A **Rust crypto trading agent** with persistent reflection memory and a double-entry audit ledger, built as a spec-driven research platform. Operates on real-data backtesting + paper simulation over top-10 USDT-quote crypto pairs (live trading is out of scope).

This README is the human entry point. AI agents should start at **[CLAUDE.md](CLAUDE.md)** then **[AGENT.md](AGENT.md)**.

---

## Status (2026-06-16)

**The active-vs-passive research program is CONCLUDED (2026-06-08): SHIP PASSIVE.**
Across all three reachable channels — price/OHLCV, derivatives-positioning, and on-chain —
no active strategy beat passive buy-and-hold net of cost. The verdict was firmed on real
bear-market data 2026-06-15 (block-bootstrap overfit-guard + a 2021-22 bear-market survey:
every apparent edge was path-fragile, not robust). The project is now in post-research
build-out / wind-down.

| Dimension | State |
|---|---|
| Mode | **Research CONCLUDED — ship passive.** Real-data backtest + paper sim. Live trading removed from scope 2026-06-12 (not wired, not planned). |
| Workspace | Rust stable, edition 2024 |
| Test gates | 119/119 anchored body-SHAs byte-identical; full lib/integration/UI-snapshot suite green. Visual-regression gate de-flaked 2026-06-16 (a multithread `set_var` race was randomizing chart renders); WCAG contrast gate flipped WARN→ENFORCING 2026-06-15. |
| UI | Cockpit shipped (`cockpit_live` + `cockpit` fixtures binaries, iced 0.14.0 + vendored `iced_tiny_skia` patch); Linux/Windows portability source shipped 2026-06-15 (macOS-verified; CI matrix deferred to the near-done milestone). |
| Strategies | SMA, composed, cross-sectional momentum, mean-reversion pairs, multi-venue, LLM-as-analyst all shipped — and all dominated by passive net of cost. |
| Strategy research retired | The full DL chain (TCN/PatchTST/GARCH/Transformer), LLM-/xgboost-/regime-forecasters, AND the derivatives perp-basis market-neutral spread (FAMILY-UNIFORM-FRAGILE 2026-06-08) + on-chain (PIT-infeasible) — i.e. the entire active-edge search. |
| Data | 10-symbol Binance hourly 2023-24 (pinned `3a8b96c4`) + a 2021-22 bear corpus added 2026-06-15 (pinned `4f390622`); fetcher made idempotent for gapped months 2026-06-16. |

For the feature-by-feature audit see [`spec/dev-notes/feature-state-table-2026-05-22.md`](spec/dev-notes/feature-state-table-2026-05-22.md); for the current wind-down reconciliation see [`spec/dev-notes/backlog-staleness-audit-2026-06-15.md`](spec/dev-notes/backlog-staleness-audit-2026-06-15.md).

---

## What this project does

**Core proposition.** A single-operator trading research stack with two differentiators:
1. **Persistent reflection memory** (`crates/reflection`): every shipped strategy decision + outcome is stored as a `LessonCard` with a 32-dim deterministic embedding, retrievable by symbol/regime via `retrieve_top_k`.
2. **Auditable double-entry ledger** (`crates/audit`): every fill, fee, slippage, LLM call, and strategy emit is recorded as journal transactions with full body-SHA-256 anchoring for byte-identical regression gates.

**Cockpit.** A native iced app (`cockpit_live`) surfaces strategy state, equity curves, drawdowns, positions, audit trail, reflection memory, and an Assistant slot for LLM reasoning traces. 14 screens; sidebar IA.

**Spec-driven workflow.** Every feature lives in `spec/<slug>/` with a brief (`feature.md`), task breakdown (`tasks.md`), decomp (`decomp.md`), and anchored backtest reports under `reports/`. Multi-agent workflow (analyst → architect → developer → tester → presenter) is documented in [AGENT.md](AGENT.md).

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

Backtest reports land under `spec/<feature>/reports/backtest-<stamp>-<scenario>.md` with body-SHA-256 anchored for regression.

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

Full audit: see [`spec/dev-notes/feature-state-table-2026-05-22.md`](spec/dev-notes/feature-state-table-2026-05-22.md) for all 54 features.

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

Retired code stays in the tree (no deletion); anchors stay locked. See [`spec/dev-notes/retired-surface-inventory-2026-05-22.md`](spec/dev-notes/retired-surface-inventory-2026-05-22.md) for the inventory.

### Infrastructure (shipped, load-bearing)

- `crates/audit/` — double-entry ledger + journal transactions + per-symbol position accounts + audit-tick stream
- `crates/reflection/` — LessonCard store + 32-dim embeddings + `top_k` retrieval + 3-state regime tagger
- `crates/llm/` — `LlmProvider` trait + Anthropic/OpenAI-compat/Ollama providers + RecordingProvider/ReplayProvider for determinism
- `crates/backtest/` — `MatchingEngine` + scenarios + report rendering with body-SHA-256 anchors
- `crates/strategy/` — `Strategy` trait + `Strategy::quantity_scale` sizing hook + all strategy implementations
- `crates/ui/` — iced cockpit (14 screens; Lab/Live/Compare/Memory/Models/Trail/Strategies/Risk/Audit/Control/Settings/Charts/Home/Debug)
- `crates/exec/` — fill publisher shim (note: real matching engine still lives in `crates/backtest/`; architect-flagged for rename per `spec/dev-notes/feature-state-architect-review-2026-05-22.md`)

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
│   ├── verify_anchors.sh   # Regression gate: 34 body-SHA-256 anchors
│   ├── spec_lint.py        # Spec structural integrity
│   ├── hash_report.py      # Canonical body-SHA hasher
│   └── check_presentation.sh
├── spec/
│   ├── product.md          # Product requirements + moat statement
│   ├── architecture.md     # System design
│   ├── architecture/       # Domain architecture + ADRs
│   │   └── adr/            # 0028-0039 architecture decision records
│   ├── backlog.md          # Active / Queue / Recent
│   ├── anchors.toml        # 34 locked body-SHA-256 regression anchors
│   ├── trace.toml          # Requirement → feature → code traceability
│   ├── dev-notes/          # Cross-cutting memos + audits + retrospectives
│   │   └── archive/2026-Q2/   # Archived stale notes
│   ├── runbooks/           # Operational runbooks
│   ├── design/             # Lumen design system
│   └── <feature-slug>/     # Per-feature folders (~54 today)
│       ├── feature.md
│       ├── tasks.md
│       ├── decomp.md (when architect M-T1 closes)
│       ├── reports/        # Anchored backtest + test reports
│       └── presentations/  # Operator-approval decks
├── target/             # Cargo build output
└── vendor/
    └── iced_tiny_skia/ # Long-term local fork (operator-locked per CLAUDE.md)
```

### Top-level files for AI agents

| File | Purpose |
|---|---|
| **[CLAUDE.md](CLAUDE.md)** | Project rules + non-negotiables + coding conventions. AI agents read this first. |
| **[AGENT.md](AGENT.md)** | Multi-agent orchestration: analyst → architect → developer ‖ ui-designer → tester → presenter loop. |
| **[spec/product.md](spec/product.md)** | What this project is and isn't (analyst-owned). |
| **[spec/architecture.md](spec/architecture.md)** | System design (architect-owned). |
| **[spec/backlog.md](spec/backlog.md)** | What's Active / Queued / Recent. |

---

## Conventions

### Status vocabulary (per `scripts/spec_lint.py` `VALID_STATUSES`)

| Status | Meaning |
|---|---|
| `draft` | Analyst sketch; no code commitment |
| `proposed` | Brief authored + ready for operator decision |
| `in-progress` | Active work (architect / developer / tester) |
| `shipped` | Code on main, anchors locked, tests green |
| `shipped-partial` | Code gates clean; one wave deferred for external-dependency reasons (first used 2026-05-22 by `v3-llm-forecaster`) |
| `retired` | Research line closed; code stays in tree; anchors locked; no further effort |
| `deprecated` | Roadmap item never built (or superseded) |
| `candidate` | Under evaluation |
| `roadmap` / `active` / `reserved` | Multi-phase initiative phases |

### Anchored body-SHA-256 regression gates

Every shipped backtest report has a body-SHA-256 entry in `spec/anchors.toml`. The gate is `bash scripts/verify_anchors.sh` (must report `ANCHORS PASS (N / N)` before any ship). Bodies are byte-immutable; documentation-link cleanup sweeps MUST exclude anchored files (see CLAUDE.md non-negotiables).

### Multi-agent workflow

Per [AGENT.md](AGENT.md): `analyst → architect → (developer ‖ ui-designer) → tester → presenter → operator-approve`. Every non-trivial change runs through this loop. The orchestrator runs sub-agents in parallel when independent. Trivial one-file edits skip the loop.

### Spec-update skill

All `spec/` file edits go through the [`spec-update`](.claude/skills/spec-update/SKILL.md) skill — never raw Write/Edit. The skill enforces frontmatter + keeps a changelog stub.

---

## Non-negotiables (from CLAUDE.md)

- No secrets in git. Keys in `config/agent.toml.local` (git-ignored) or env vars per `spec/architecture.md`.
- No shipping on a `REGRESSION` verdict without an explicit human override.
- No silent divergence from `spec/architecture.md`.
- **Every strategy overlay or sizing-modifier ships with a baseline-equity-divergence e2e test from day 1** (precedent: `v3-volatility-forecaster-noop-fix` 2026-05-22; the noop bug went undetected by 5 sequential gates).
- **Anchored report files in `spec/*/reports/` are byte-immutable** per ADR-0038 § D6; even mechanical link-fix edits mutate the body-SHA. Documentation-link cleanup MUST exclude anchored files.

---

## Key dev-notes

When orienting on the project, these dev-notes give the most signal per word:

- [`feature-state-table-2026-05-22.md`](spec/dev-notes/feature-state-table-2026-05-22.md) — comprehensive 54-feature inventory
- [`v25-dl-journey-retrospective-2026-05-22.md`](spec/dev-notes/archive/2026-Q2/v25-dl-journey-retrospective-2026-05-22.md) — what the forecaster track taught (4 retirements)
- [`v3-vol-overlay-noop-discovery-2026-05-22.md`](spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md) — the load-bearing engineering pattern
- [`strategic-reset-2026-05-23.md`](spec/dev-notes/archive/2026-Q2/strategic-reset-2026-05-23.md) — half-validated moat finding + next-6-week roadmap framing
- [`feature-state-analyst-review-2026-05-22.md`](spec/dev-notes/archive/2026-Q2/feature-state-analyst-review-2026-05-22.md) + [`feature-state-architect-review-2026-05-22.md`](spec/dev-notes/archive/2026-Q2/feature-state-architect-review-2026-05-22.md) — dual-perspective reviews

---

## License

Private / single-operator research project. No external license at this time.
