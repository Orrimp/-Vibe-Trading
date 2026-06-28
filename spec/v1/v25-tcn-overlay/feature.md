---
slug: v25-tcn-overlay
status: shipped
owner: operator
updated: 2026-06-17
version: 2.5.0
parent: v25-dl-forecast-overlay v2.5.0 (deprecated 2026-05-22)
predecessor: v2-llm-strategy v2.0.0
shipped_disposition: F4 verdict — BS-1/BS-2 anchored checkpoints + 4 strategy anchors + 4 realdata anchors all delivered, but confidence-gate forecaster does not extract +0.10 Sharpe-delta vs v1 baseline. Strategy crate remains on disk for paper-mode + opt-in advisory builders (`with_tcn_bs{1,2}_ledger_tuned(τ, ε)` shipped at v25-tcn-threshold-tuning v0.1.0). Production deployment NOT recommended. See `spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md` for the full evidence chain.
---

# v2.5 — TCN forecast overlay (phase 1 of 4)

**shipped — compressed 2026-06-17.** One-line description and version: see [CHANGELOG.md](../../../CHANGELOG.md). Full narrative history: `git log -- spec/v25-tcn-overlay/`. Backtest evidence (if any) is preserved under `reports/`.
