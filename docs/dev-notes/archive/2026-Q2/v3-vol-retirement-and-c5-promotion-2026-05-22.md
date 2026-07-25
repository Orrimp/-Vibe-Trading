---
slug: v3-vol-retirement-and-c5-promotion-2026-05-22
date: 2026-05-22
authors: operator + orchestrator
related:
  - spec/v3-volatility-forecaster/feature.md
  - spec/v3-volatility-forecaster-rebaseline/feature.md
  - spec/v3-volatility-forecaster-noop-fix/feature.md
  - spec/v3-volatility-forecaster-noop-fix/presentations/v3-volatility-forecaster-noop-fix-2026-05-22.md
  - spec/v3-llm-forecaster/feature.md
  - spec/v3-regime-classifier/feature.md
  - docs/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md
  - docs/dev-notes/v25-dl-journey-retrospective-2026-05-22.md
  - docs/dev-notes/strategy-reformulation-survey-2026-05-22.md
---

# v3 volatility-forecaster programme retirement + C5 promotion — 2026-05-22

Operator decision logged 2026-05-22 under the v3-volatility-forecaster-noop-fix v0.1.0 sprint-review deck approval. This dev-note captures the routing decision + its downstream effects so future-operator reviews and audit passes don't have to retrace the 5-feature reasoning chain.

## What got retired

Programme-retired (`status: retired`):

- `v3-volatility-forecaster` v0.1.0 — Candidate 1 of the 2026-05-22 strategy-reformulation HYBRID pick.
- `v3-volatility-forecaster-rebaseline` v0.1.0 — the operator's parent-deck (b) RE-BASELINE FIRST follow-on.

Both shipped with passing code gates + locked anchors and remain in the tree as evidence. No code deletion; no anchor unlock; no commit revert. The `[v3.0.0-volatility*]` namespaces in `spec/anchors.toml` carry 4 anchored body-SHAs as the regression contract.

## Why

Joint advisory under the v3-volatility-forecaster-noop-fix v0.1.0 fix wave:

| Metric | Pre-fix (no-op overlay) | Post-fix (real wiring) |
|---|---|---|
| Final equity (2023 full year) | $113,479.98 | **$62,807.89** (−44.6%) |
| net_delta vs un-targeted real-baseline | 0.000000 | **−0.021719** |
| V-verdict | V3 (calibration ratio 2.952191) | V3 (unchanged) |
| Joint advisory | MODEL-BROKEN / NO-ALPHA (artifactual) | **MODEL-BROKEN / NO-ALPHA / NEGATIVE-NET-DELTA** (real) |

The mechanism is architecturally explainable: GARCH under-predicts realized vol by ~2.95× → the overlay's `target_vol / sigma_hat` ratio is inflated 2.95× → upper clamp at `2.0×` activates on most bars → the strategy effectively runs at ~2× leverage → drawdowns are amplified by the same factor on a universe with ~73% historical max DD.

The (c) DEBUG V3 / (d) v0.1.1 GARCH-refit salvage paths were foreclosed by the structural mechanism: even perfectly-calibrated GARCH would best-case de-leverage the overlay back to the un-targeted baseline (net_delta → 0), at the cost of multi-week effort.

## Discovery chain (cross-link)

The retirement decision was enabled by the operator's caveman-probe routing — see `docs/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md` for the full diagnostic chain (σ_hat × 2.95 produces byte-identical equity → code review of `vol_targeting_overlay.rs:309-319` → comment "diagnostic only — the backtest engine reads quantities from fills, not from signal metadata").

The parent (b) RE-BASELINE FIRST routing pick from earlier that day was *fortuitously* correct ((a) RETIRE is the same conclusion as the rebaseline-deck's recommendation), but for the wrong reason — the rebaseline alone could not have surfaced the no-op bug; the caveman probe + code review were the load-bearing diagnostic step.

## What got promoted

Promoted Queue → Active (`status: proposed`):

- `v3-llm-forecaster` (Candidate 5) — operator picked C5 over C2 for:
  - Moat alignment (`product.md § Differentiator` lines 79-83: persistent reflection memory + auditable double-entry ledger).
  - Infrastructure reuse (existing `crates/llm/` LLM + replay-cache).
  - Information-theoretic independence from the v2.5 F4-F4-F4 + v3 vol chain (LLM-as-forecaster doesn't share signal sources with TCN/PatchTST/GARCH).

Stays in Queue (`status: draft`): `v3-regime-classifier` (Candidate 2). Not de-prioritised; just not the next pick.

## What's next for C5

The C5 analyst pass was **spec-only design exploration** (R1-R10, H1-H5, Q1-Q8, K1-K10, 8-item non-regression contract, deferred-milestone activation contract). It does NOT have a populated `tasks.md` with OD/AR/D-N/T-T/T-P rows.

The next step is an **analyst-bridge pass** to:
1. Author `spec/v3-llm-forecaster/tasks.md` mirroring the analyst-pass tasks of prior features (T-A* analyst rows ticked; T-OD* operator-decide rows; T-AR* architect stubs; T-D-N* developer stubs; T-T* tester stubs; T-P1 presenter stub).
2. Resolve any open Q1-Q8 that have analyst-default rationales not already locked in feature.md.
3. Flip `spec/trace.toml` REQ-V3-LLM-FORECASTER-001 state from `draft` → `proposed` (matching the new feature.md status).
4. Emit handoff to the operator for OD ticks (standing Autoapprove may apply to a subset).
5. Hand off to architect M-T1.

Expected analyst-bridge budget: ~30-60 minutes given the spec-only analyst pass already exists.

## Retirement contract (precedent)

This is the **second** programme retirement in the v3 strategy-reformulation track (v25-dl-forecast-overlay was the first, 2026-05-10). The retirement contract is:

- Code stays in the tree.
- Anchors stay locked.
- `status` flips to `retired` with a `retired_2026_xx_xx:` frontmatter marker citing the deck + operator decision.
- Backlog entry moves to "Recent (shipped, retired)" cohort. (Backlog cleanup is a spec-auditor task; not load-bearing for this dev-note.)
- A dev-note like this one captures the operator decision + cross-links so future audits don't retrace.

Future re-investigations of the same axis (vol forecasting) are NOT forbidden — they would just need a new feature folder + a new ADR if the strategy structure differs materially from v0.1.0.

## Cross-references

- The retirement deck: `spec/v3-volatility-forecaster-noop-fix/presentations/v3-volatility-forecaster-noop-fix-2026-05-22.md`.
- The fix-wave tester report: `spec/v3-volatility-forecaster-noop-fix/reports/test-final-2026-05-22.md` (VERDICT → PASS; 34/34 anchors).
- The bug discovery chain: `docs/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`.
- The prior programme retirement: `docs/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`.
- The strategy-reformulation survey that ranked C1/C2/C5: `docs/dev-notes/strategy-reformulation-survey-2026-05-22.md`.
- The promoted C5 brief: `spec/v3-llm-forecaster/feature.md`.
- ADR-0038 § D6.b — anchor re-emission protocol (first documented use: the v3-volatility-forecaster-noop-fix retirement chain).
