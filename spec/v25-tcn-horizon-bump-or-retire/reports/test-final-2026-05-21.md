---
title: v25-tcn-horizon-bump-or-retire v0.1.0 — decision-record verification
date: 2026-05-21
feature: v25-tcn-horizon-bump-or-retire
version: 0.1.0
verdict: PASS-DECISION-RECORD
shape: decision-record-stub (audit-2026-05-22 P2.6)
---

# Decision-record verification — v25-tcn-horizon-bump-or-retire v0.1.0

## Why a stub rather than a tester report

This feature is a **policy/decision feature**, not a code-change feature.
It ships zero code modifications:

- **`crates` array in trace.toml**: `[]`
- **`tests` array in trace.toml**: `[]`
- **`anchors` array in trace.toml**: `[]`
- **Net-new files in `crates/`**: zero
- **Modified files in `crates/`**: zero

The substantive deliverable was an **operator-decide artifact** —
specifically the resolution of Q1 (primary scope) at M-OD, which had
no safe analyst default and could not be auto-approved. Operator chose
**Q1 = (b) RETIRE v2.5 TCN at 1h horizon; pivot multi-week budget to
v2.5a PatchTST**. Q2-Q7 became MOOT under (b) (no retrain → no horizon
target, no checkpoint count, no topology, no data span, no retire-on-F4
threshold, no anchor strategy).

There is no software artifact to test. There is a decision artifact to
verify, which is what this report does.

## Verification

| Item | Method | Result |
|------|--------|--------|
| Operator Q1 resolution recorded | `grep '^- \\[x\\] T-OD1' tasks.md` | PASS — `T-OD1 — Q1 = (b)` ticked 2026-05-21 |
| Q2-Q7 MOOT explicitly documented | `tasks.md § M-OD` rows T-OD2..T-OD7 | PASS — all 6 ticked with "MOOT under Q1=(b)" rationale |
| `feature.md` status flipped | `head -10 feature.md` | PASS — `status: shipped, owner: operator, version: 0.1.0` |
| `tasks.md` status flipped | `head -10 tasks.md` | PASS — `status: shipped, owner: operator` |
| `trace.toml` REQ row state | `grep -A 1 "REQ-V25-TCN-HORIZON-BUMP-OR-RETIRE-001" trace.toml` | PASS — `state = "shipped"` |
| Backlog migration Active→Recent | `grep "horizon-bump-or-retire" backlog.md` | PASS — entry in `## Recent (shipped)` section |
| Activation flag for follow-on | `grep "ACTIVATION TRIGGERED" backlog.md` | PASS — `v25a-patchtst-overlay` activation flag set in Queue § Strategy at the time of v0.1.0 ship; subsequently promoted Queue → Active 2026-05-21 by analyst pass |
| Anchor non-regression invariant | `bash scripts/verify_anchors.sh` 2026-05-21 | PASS — 28/28 (no anchor changes; this feature is anchor-additive-zero) |
| ADR ledger | review at `spec/architecture/adr/README.md` | PASS — no new ADR (decision-record only; ADR-0035 σ_train + ADR-0033 F-verdict carry-forward as cross-phase invariants per the operator's routing rationale) |
| Spec-lint baseline preservation | `python3 scripts/spec_lint.py` | PASS — 87 / 2 categories at ship-time = predecessor baseline; zero new categories introduced by this feature |

## Downstream effect verification

The operator's Q1=(b) decision triggered:

1. **v25a-patchtst-overlay activation** — Queue § Strategy → Active 2026-05-21 by analyst pass; v0.1.0 shipped 2026-05-22 with joint F4 verdict (Sharpe-delta +0.006 vs v1 baseline) confirming the operator's "1h TCN doesn't extract alpha" reading.

2. **No horizon-bump retrain spawned** — Wave-A/B/C/D never authored in this feature folder. The architect placeholders in `tasks.md § Architect rows (T-AR)` remain unticked under "moot under Q1=(b)".

3. **Multi-week budget pivot** — the operator routed the freed compute time to PatchTST training (~7.75 hours actual wall-clock vs ~2-3 weeks estimated horizon-bump retrain). Substantial wall-clock and compute saving. The PatchTST F4 result subsequently validated the operator's retirement choice — a horizon-bumped TCN was unlikely to outperform the patch-attention paradigm.

4. **Strategic retirement of the full DL roadmap** — 2026-05-22, after v25a-patchtst-overlay shipped F4. v25-dl-forecast-overlay umbrella + v25b-transformer-overlay + v26-forecast-bakeoff all flipped `roadmap → deprecated`. This downstream retirement was *enabled* by the operator's earlier decisive Q1=(b) routing; without it, the multi-week TCN retrain would have happened first and the v2.5b/v2.6 work would have queued behind it.

## What this report is NOT

- Not a tester `VERDICT → PASS` (no code → no test gates to run)
- Not a presenter deck (feature.md serves as both brief + decision record)
- Not an anchor lock (anchors unchanged at 28 across this feature)
- Not a CI gate (no executable produced)

## Cross-references

- Feature brief + decision: `spec/v25-tcn-horizon-bump-or-retire/feature.md`
- Tasks (T-OD ticks): `spec/v25-tcn-horizon-bump-or-retire/tasks.md § M-OD`
- Predecessor evidence chain (3 substantive ships):
  - `spec/v25-tcn-alpha-investigation/` (F4 verdict)
  - `spec/v25-tcn-recalibrate/` (σ_train fix)
  - `spec/v25-tcn-threshold-tuning/` (T-MARGINAL)
- Downstream consequence: `spec/v25a-patchtst-overlay/` (F4 confirmed retirement rationale)
- Retrospective: `spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`
- Audit that flagged the missing report: `spec/dev-notes/audit-2026-05-22.md § P2.6`

## Auditor note

This stub was authored 2026-05-22 to satisfy the `shipped-no-tests`
spec-lint contract item flagged at audit-2026-05-22 P2.6. The contract
allows decision-record features to ship without test gates provided
they emit a verification report documenting the decision artifact +
downstream effect chain. This report fulfils that contract.
