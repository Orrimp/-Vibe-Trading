---
slug: v3-xgboost-cheap-classifier
status: draft
owner: analyst
updated: 2026-05-29
---

# Tasks — v3-xgboost-cheap-classifier v0.1.0

> Analyst M0 rows complete 2026-05-29 (Queue pre-position). Architect /
> developer / tester rows are **DEFERRED placeholders** per Route A
> pre-position — no work past M0 until operator picks Route A
> Candidate 6 from
> [`spec/dev-notes/post-v3-strategy-direction-2026-05-29.md`](../dev-notes/archive/2026-Q2/post-v3-strategy-direction-2026-05-29.md).

## M0 — Analyst (DONE 2026-05-29)

- [x] **M0-1** — Queue pre-flight: no existing `spec/v3-xgboost-*`
  folder with shipped status. Greenfield.
- [x] **M0-2** — Read predecessor inputs (post-v3 dev-note Route A;
  survey § Candidate 6; C1/C2/C5 records; frozen trait seam).
- [x] **M0-3** — Author `feature.md` (R1-R5 + R-NR + K1-K6 + H1-H3 +
  Q1-Q3 + 4-cell verdict tree + cost framing both routes).
- [x] **M0-4** — Author `tasks.md` (this file).
- [x] **M0-5** — Append `[[req]]` row `REQ-V3-XGBOOST-001` at EOF of
  `spec/trace.toml` in `proposed` state.
- [x] **M0-6** — Append Queue § Strategy entry to `spec/backlog.md`
  with explicit "Queue NOT Active" annotation.
- [x] **M0-7** — Verify gates: `verify_anchors.sh` 75/75 PASS;
  `spec_lint.py` baseline-stable.

## M-OD — Operator-decide (DEFERRED until Route A pick)

- [ ] **M-OD-1** — Operator picks Route A from post-v3 dev-note.
- [ ] **M-OD-2/3/4** — Operator answers Q1/Q2/Q3
  (analyst-recommended all (a) DURABLE).

## M-T1 — Architect (DEFERRED)

- [ ] **M-T1-1** — Ratify Q1-Q3 per M-OD.
- [ ] **M-T1-2** — K1 pre-flight: `cargo build -p forecast --features
  xgboost` on Apple Silicon. If fails → Q3=(b) re-pick.
- [ ] **M-T1-3** — Decompose Wave A→D; sibling-ADR vs new ADR
  (expect ADR-0049 § Changelog amendment only).
- [ ] **M-T1-4** — Pick anchor namespace (rec. `v3.5.0-xgboost`).
- [ ] **M-T1-5** — Verify R2 trait reuse (XGBoost impl satisfies
  frozen `RegimeClassifier` trait without amendment).

## M-DEV — Developer Waves A-D (DEFERRED)

- [ ] **Wave A** — Classifier core; new `crates/forecast/src/xgboost.rs`
  satisfying `RegimeClassifier` trait. ~5-7 days.
- [ ] **Wave B** — Overlay strategy + **mandatory** R-NR.7 e2e
  divergence test from day 1 per CLAUDE.md + K6. ~3-5 days.
- [ ] **Wave C** — 2 backtest scenarios + 2 anchors under
  `v3.5.0-xgboost`. ~3-5 days.
- [ ] **Wave D** — 2024 held-out validation + H1 accuracy report.
  ~2-3 days.

## M-FINAL — Tester (DEFERRED)

- [ ] **M-FINAL-1** — Standard test-report.md.
- [ ] **M-FINAL-2** — V-verdict (V-XGB-PASS / CLASSIFIER-ONLY /
  DAMPENED / INCONCLUSIVE).
- [ ] **M-FINAL-3** — R-NR.7 divergence + K3 class-distribution
  gates; 75 → 77 anchors PASS.

## M-P — Presenter (DEFERRED)

- [ ] **M-P-1** — Sprint-review deck at `presentations/<slug>-<date>.md`.
