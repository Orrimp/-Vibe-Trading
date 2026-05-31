---
slug: carry-strategy
status: draft
owner: analyst
updated: 2026-05-31
---

# Tasks — carry-strategy

Analyst scope complete (`feature.md`, draft). **Architect M-T1 is gated on
operator confirmation of the build** — the analyst's honest estimate is
~4.5–7.5 days (3–5× MR), driven by the funding-through-bootstrap
integration crux (Sub-C), not the data (already banked at `ab815d5`).

- [x] M-A1 — analyst scope: signal + sign convention, integration design, size estimate, recommendation (long-only v0.1.0).
- [ ] M-T1 — architect: resolve Q-CARRY-1..5 (the funding→strategy seam, long-only vs market-neutral, funding-through-bootstrap, score normalization, lock the θ-grid) + the ADR-0051 § D6.6 amendment. **Pending operator greenlight.**
- [ ] M-DEV — develop the funding loader + bootstrap resampling + funding-cashflow accrual + the carry signal + the day-1 both-axes gate.
- [ ] M-TEST — verify on both robustness axes vs the +1.74 buy-and-hold bar.
