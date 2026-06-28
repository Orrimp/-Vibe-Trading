---
slug: v25-dl-forecast-overlay
status: deprecated
owner: operator
retired: 2026-05-22
retired_reason: Joint F4-F4-F4 verdict across TCN BS-1 @ 1h (+0.018 Sharpe-delta), TCN BS-2 @ 1h (+0.045), PatchTST BS-1 @ 24h (+0.006) — all below +0.10 T-ALPHA-UNLOCKED threshold. Operator routing (a) at v25a-patchtst-overlay v0.1.0 ship 2026-05-22 retires the entire 4-phase DL umbrella. Phase 1 (TCN) shipped + retired-at-1h. Phase 2 (PatchTST) shipped + scored LOWER than retired phase 1. Phases 3 (v25b vanilla Transformer) + 4 (v26 bake-off) retired without shipping — prior probability of phase 3 unlocking +0.10 Sharpe-delta given F4-F4 evidence is below the ~3-5 week compute budget threshold.
updated: 2026-06-17
version: 2.5.0
predecessor: v2-llm-strategy v2.0.0
supersedes: v25-kronos-forecast-overlay (dropped 2026-05-16 — see ADR-0028)
---

# v2.5 — DL forecast overlay (4-phase roadmap)

**deprecated — compressed 2026-06-17.** One-line description and version: see [CHANGELOG.md](../../../CHANGELOG.md). Full narrative history: `git log -- spec/v25-dl-forecast-overlay/`. Backtest evidence (if any) is preserved under `reports/`.
