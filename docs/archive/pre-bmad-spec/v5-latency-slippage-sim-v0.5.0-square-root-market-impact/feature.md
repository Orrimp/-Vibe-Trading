---
slug: v5-latency-slippage-sim-v0.5.0-square-root-market-impact
version: 0.2.0
status: shipped
owner: developer
updated: 2026-06-17
predecessor: v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit v0.1.0
parent: backtest-vs-live-execution-gap
priority: P1
q_d1: "(a) Linear{bps:8} fallback for synthetic scenarios — operator ratified 2026-05-29"
q_d2: "(β) Per-scenario lazy-compute via universe_avg_daily_volume_usd_trailing — operator ratified 2026-05-29"
anchor_cascade_revised: "75 → 84 (9 new real-data anchors under v5-sqrt-impact-2026-05; brief described 10 scenarios but top10-2024-fy-momentum-realdata was never implemented — only 2023 counterpart shipped)"
m_od_q3b_supersession: "M-OD 2026-05-29 Q3=(b) ratification SUPERSEDED by Q-D1=(a); see docs/dev-notes/v5-v0.5.0-q-d1-q-d2-decision-brief-2026-05-29.md"
---

# v5 latency-slippage-sim v0.5.0 — square-root market-impact model

**shipped — compressed 2026-06-17.** One-line description and version: see [CHANGELOG.md](../../CHANGELOG.md). Full narrative history: `git log -- spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/`. Backtest evidence (if any) is preserved under `reports/`.
