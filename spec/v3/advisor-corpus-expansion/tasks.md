---
slug: advisor-corpus-expansion
status: proposed
owner: architect
updated: 2026-07-09
---

# Tasks — P2 corpus expansion + verdict re-run

> **Analyst seed (M-T0, 2026-07-09).** This is a placeholder task spine so the
> feature is not an orphan while it sits in `proposed`. The **architect owns
> the real task breakdown** at M-T1 — after resolving Q-CE-1..7 (esp. Q-CE-2
> the second-venue decision and Q-CE-3 the exogenous back-fill scope), the
> windows/subsets in R1, and the re-run harness seam (Q-CE-7). Fetching happens
> only AFTER the architect designs (per the brief's hard constraint). See
> [feature.md](feature.md) for Why / Requirements R1-R8 / open questions.

## Architect (M-T1) — design pass

- [ ] AT1 — resolve Q-CE-1..7; lock the second-venue choice (Coinbase-hourly recommended vs Kraken-daily fallback) and the exogenous back-fill scope — _acceptance: each Q-CE answered in feature.md § Design with a rationale._
- [ ] AT2 — confirm the R1 corpus windows + honest per-window symbol subsets + the 2526 last-closed-month clamp — _acceptance: a final corpus table (dir × window × symbols) ratified in § Design._
- [ ] AT3 — pick the re-run harness seam (dedicated `p2_verdict_rerun` mirroring `null_data_no_crown.rs` vs a `--corpus <dir>` selector) + confirm the ADR scope (analyst lean: small corpus-set + Coinbase-adapter ADR, no gate change) — _acceptance: seam named, ADR number reserved if owed._
- [ ] AT4 — hand the developer task list (fetch order, exogenous back-fills, the re-run matrix, the consistency/smoke tests, the AC1-AC8 report) — _acceptance: developer-owned tasks appended below._

## Developer / tester — filled after M-T1

_developer + tester populate after the architect design pass. Placeholder;
the fetch + re-run + report tasks land here once the venue + windows are locked._

## Notes

- Anchors 119/119 + spec-lint PASS(0) gated per commit; `write_report=false`
  throughout the re-run (anchor-safe by construction); FROZEN gate byte-frozen;
  existing pinned corpora SHAs byte-immutable.
