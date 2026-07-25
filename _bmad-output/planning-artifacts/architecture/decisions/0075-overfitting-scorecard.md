---
adr: 0075
title: Overfitting scorecard — report-only DSR + MinBTL + N_eff alongside every bake-off verdict; gate byte-untouched
status: accepted
date: 2026-06-30
supersedes: none
superseded-by: none
---

# ADR-0075: The overfitting scorecard (P0-1)

## Context

The product's credibility thesis is *"traceable & plausible"* — nine
independent research reviews (900 papers) concluded that no single-coin
active strategy reliably beats holding net of costs. The bake-off + the
FROZEN robustness gate already produce a verdict; what the cockpit lacked
was a visible answer to the one question every operator asks of a
"best-of-N" pick: **"did we fool ourselves by trying many strategies?"**

The v2 architect (`spec/v2/v2-architecture.md` §1 P0-1) scoped this as the
canonical **report-annex registration seam** — one of the three latent
seams formalized instead of building a runtime plugin host (§3 verdict:
"NO plugin architecture. Stay additive.").

## Decision

Land a **report-only** overfitting scorecard alongside every bake-off
`Recommendation` — never a re-rank, never a veto.

### Surface

- A new pure module `crates/backtest/src/bakeoff/scorecard.rs` computes
  four closed-form numbers from inputs the bake-off already has (the
  per-candidate Sharpe vector + the crown's bootstrap distribution +
  the crown's return skew/kurtosis):

  | Field | Formula | Source |
  |---|---|---|
  | `n_candidates: usize` | Raw field size | bake-off slate |
  | `n_eff: f64` | `ρ̄ + (1 − ρ̄) · M` (closed form) | `backtesting[1] App.3` |
  | `deflated_sharpe: f64` | `Φ[(ŜR − SR₀)·√(T−1) / √(1 − γ̂₃·ŜR + ((γ̂₄−1)/4)·ŜR²)]` with `SR₀ = √V·((1−γ)Φ⁻¹[1−1/N] + γΦ⁻¹[1−(1/N)e⁻¹])` | Bailey & López de Prado, `evolution[98]` |
  | `min_btl_years: f64` | `2·ln(N_eff) / SR²_target` | `evolution[29]`, `ml-trading[95]` |
  | `pbo: Option<f64>` | **`None` in v2** (deferred to the Tune surface) | `v2-architecture.md` §6.0 D1 |
  | `crown_clears_dsr: bool` | `DSR ≥ 0.95` (**informational only**) | `v2-architecture.md` §6.0 D3 |

- Carried on `Recommendation.scorecard: Scorecard`. Mirrored to `ui` via
  `BakeoffReportMirror::from_report` → `ScorecardView` as plain
  `usize`/`f64`/`bool`. **Zero new `ui` dep edge.**

- A leaderboard *"How much to trust this"* panel below the ranked table:
  four facts (`STRATEGIES TRIED`, `DEFLATED CONFIDENCE`, `MINIMUM HISTORY
  NEEDED`, `BEATS HOLDING AFTER THE SEARCH?`) each with a plain-language
  gloss. Reuses `frame::panel` (zero new widgets, zero new theme tokens).

- The same `Scorecard` is later projected as `ScorecardSummary` and
  carried on `ForwardPlan.confidence` (P0-3, ADR-0076 sibling).

### Anchor safety

The advisor bake-off path runs `write_report=false` → the scorecard is
**anchor-safe by construction**. `verify_anchors.sh` stays 119/119
before and after.

### Frozen-gate identity

`scorecard_does_not_change_ranking` (unit test in `scorecard.rs`) asserts
`rank_candidates` produces byte-identical `crowned`, `outcome`, and
`order` before and after the scorecard is computed. The FROZEN
`verdict_bands` + `classify_verdict` never read the scorecard.

### D1–D4 ratifications (operator, 2026-06-28, `v2-architecture.md` §6.0)

- **D1 PBO timing** — closed-form DSR/MinBTL/N_eff first; PBO deferred to
  the homogeneous Tune/sweep surface where CSCV is statistically
  meaningful. `compute_scorecard_pbo_always_none` is the v2 invariant.
- **D2 DSR threshold** — no fixed cutoff; the haircut is reported, not a
  binary. ORATIO threshold derivation deferred until a future veto.
- **D3 Crown-eligibility veto** — **report-only in v2**. `crown_clears_dsr`
  is informational; the field is the **one-line switch** a future veto
  would flip. A veto is a FROZEN-gate change and needs its own ADR + an
  operator call.
- **D4 N_eff method** — **closed-form, frozen** at `MAX_SWEEP_CONFIGS = 24`.
  T ≫ 24 on any bootstrappable window, so the literature's "must cluster
  first when M>T" rule does not apply. The freeze closes the door on
  second-order snooping (CX-2).

## Status: accepted

- Tester `VERDICT → PASS` at commit `1d5b114`
  (`spec/v2/advisor-overfitting-scorecard/reports/test-2026-06-29-advisor-overfitting-scorecard.md`).
- 759 tests pass (incl. 16 scorecard unit tests + the gate-identity test).
- Anchors 119/119 byte-immutable; spec-lint PASS; clippy `-D warnings` clean.
- Operator-approved 2026-06-30 — status flipped `tester-done → shipped`.

## Consequences

- **The Honesty Scorecard is the canonical report-annex seam.** New
  scorecard metrics in future v2 increments add fields to `Scorecard` and
  mirror fields to `ScorecardView` — no new module required.
- **A future DSR/PBO veto remains possible** without re-architecting; the
  `crown_clears_dsr` carrier is already in place. The change would be a
  single line in `rank_candidates` *plus* a new ADR + operator call.
- **Per-row tail expansion is unblocked** (P1-2 surfaced the crown's tail
  only; future polish can add per-candidate tooltips reading the same
  `PathMetrics` vector).
- **PBO on the Tune surface** is the natural next increment for the
  scorecard pillar — the architect's §1 P0-1 explicitly reserves the slot.

## References

- Spec: `spec/v2/advisor-overfitting-scorecard/feature.md`.
- Tester report: `spec/v2/advisor-overfitting-scorecard/reports/test-2026-06-29-advisor-overfitting-scorecard.md`.
- Presenter deck: `spec/v2/advisor-overfitting-scorecard/presentations/advisor-overfitting-scorecard-2026-06-29.md`.
- Architect: `spec/v2/v2-architecture.md` §1 P0-1 + §3 + §6.0 D1–D4.
- Research: `research/backtesting/application-overfitting-and-multiple-testing.md`;
  `research/evolution/application-anti-overfitting-and-search-discipline.md`.
- Commits: `9c3c002` (backend) · `ac7c779` (UI) · `d3a9a4a` (arm-count refresh)
  · `1d5b114` (tester VERDICT PASS) · `470b871` (presenter deck).
