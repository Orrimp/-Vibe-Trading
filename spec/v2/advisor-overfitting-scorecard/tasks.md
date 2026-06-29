---
slug: advisor-overfitting-scorecard
status: dev-done
owner: developer
version: 0.1.0
updated: 2026-06-29
---

# Tasks — Overfitting Scorecard (P0-1)

## Developer (backend) — DONE

- [x] `bakeoff/scorecard.rs` pure module: `Scorecard` struct (`n_candidates`, `n_eff`,
      `deflated_sharpe`, `min_btl_years`, `pbo: Option<f64>` = None, `crown_clears_dsr`).
- [x] Closed forms: `n_eff` = `ρ̄+(1−ρ̄)·M` (frozen at 24-config scale); `min_btl` ≈
      `2·ln(N)/SR²`; `dsr` (skew + total-kurtosis term); high-accuracy `normal_inv_cdf`
      (Acklam + Halley) + `normal_cdf`; skew / excess-kurtosis / `sharpe_variance`.
- [x] `Recommendation.scorecard` carrier + compute in `run_bakeoff` (report-only).
- [x] Tests: N_eff (correlated→1 / uncorrelated→M / single / empty), MinBTL (formula,
      N=24, N≤1), DSR research worked examples (fails-at-N100 / passes-at-N46 /
      clears-at-N88-Normal), `normal_inv_cdf` roundtrip, **gate-identity** (`rank_candidates`
      byte-identical with/without the scorecard).
- [x] Gates: `cargo test -p backtest --lib scorecard` (16 pass) · clippy `-D warnings` ·
      `cargo fmt` · `verify_anchors.sh` 119/119.

## Follow-on (separate increments)

- [ ] ui-designer: `ScorecardView` mirror in `BakeoffReportMirror::from_report` +
      the leaderboard scorecard block + `crate::strings` copy (no new `ui` dep edge).
- [ ] tester: full `cargo test -p backtest` regression + the VERDICT report.
- [ ] ADR-0075 authored + registered atomically.
- [ ] PBO/CSCV on the Tune/sweep surface (D1, later increment).
