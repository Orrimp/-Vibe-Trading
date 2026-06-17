---
slug: v3-regime-classifier
version: 0.1.0
status: shipped
owner: shipped
shipped_disposition: T-REG-NO-ALPHA verdict — net Sharpe-delta -0.294113 vs un-overlaid v1 momentum baseline on 2024 held-out validation; V-REG-5 verdict (classifier fails to separate regimes meaningfully). Operator R-O 2026-05-29 → RETIRE + close v3 three-pick set. Anchored Wave E bodies stay as scientific record. Production deployment NOT recommended.
updated: 2026-06-17
predecessor: spec/dev-notes/strategy-reformulation-survey-2026-05-22.md (Candidate 2)
parent: v3-three-pick
priority: P2
promoted_2026_05_28: Queue → Active by operator under the v2.5 TCN re-investigation halt routing (TCN line correctly retired 2026-05-21; C1 v3-volatility-forecaster retired 2026-05-22 NEGATIVE-NET-DELTA; C5 v3-llm-forecaster shipped v0.1.0-PARTIAL 2026-05-22). C2 is the remaining v3 three-pick slot. M-A5 light-touch refresh per the 2026-05-22 deferred-milestone activation contract.
sibling_picks:
  - v3-volatility-forecaster (Candidate 1; RETIRED 2026-05-22 NEGATIVE-NET-DELTA)
  - v3-llm-forecaster (Candidate 5; shipped v0.1.0-PARTIAL 2026-05-22)
---

# v3 — Regime classifier (predict regime label, not μ)

**shipped — compressed 2026-06-17.** One-line description and version: see [CHANGELOG.md](../../CHANGELOG.md). Full narrative history: `git log -- spec/v3-regime-classifier/`. Backtest evidence (if any) is preserved under `reports/`.
