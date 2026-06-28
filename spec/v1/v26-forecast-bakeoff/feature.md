---
slug: v26-forecast-bakeoff
status: deprecated
owner: operator
retired: 2026-05-22
retired_reason: Bake-off premise (head-to-head TCN vs PatchTST vs v25b vanilla Transformer) was the canonical retirement gate for the 4-phase DL umbrella. Joint F4-F4-F4 verdict across v25-tcn-overlay (BS-1/BS-2 @ 1h: +0.018/+0.045 Sharpe-delta) + v25a-patchtst-overlay (BS-1 @ 24h: +0.006) is itself the bake-off result — the bake-off is moot when 2 of 3 paradigms have already F4'd. Operator routing (a) at v25a v0.1.0 ship 2026-05-22 retires v26 without shipping. Stub feature folder preserved for archeology.
updated: 2026-06-17
version: 2.6.0
parent: v25-dl-forecast-overlay v2.5.0 (roadmap)
predecessor: v25b-transformer-overlay v2.5.2
---

# v2.6 — Forecast bake-off + retirement (phase 4 of 4)

**deprecated — compressed 2026-06-17.** One-line description and version: see [CHANGELOG.md](../../../CHANGELOG.md). Full narrative history: `git log -- spec/v26-forecast-bakeoff/`. Backtest evidence (if any) is preserved under `reports/`.
