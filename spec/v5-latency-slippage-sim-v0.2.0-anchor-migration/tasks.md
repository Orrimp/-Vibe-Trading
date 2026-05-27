---
slug: v5-latency-slippage-sim-v0.2.0-anchor-migration
status: draft
owner: analyst
updated: 2026-05-27
priority: P1
---

# v5 v0.2.0 anchor migration — tasks

> Inline-salvaged 2026-05-27 from analyst `ac4d192d801af160a` which
> 529'd at 14 tool-uses (wrote feature.md then dropped before tasks.md).
> Standard 6-milestone scaffold per AGENT.md.

## M0 — Analyst

_owner: analyst_

- [x] **T-A1** (2026-05-27) — `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md`
  v0.1.0 authored with R1-R7 + R-NR + K1-K5 + H1-H3 + Q1-Q4 +
  pre-drawn 4-cell verdict tree + cost framing + cross-references.
- [x] **T-A2** (2026-05-27) — tasks.md scaffold (this file; inline-salvaged
  by orchestrator after analyst 529'd).
- [ ] **T-A3** — Active row appended to
  [`spec/backlog.md`](../backlog.md). (Orchestrator inline-completes.)
- [ ] **T-A4** — Trace row `REQ-V5-ANCHOR-MIGRATION-V0-2-0-001`
  appended at END of [`spec/trace.toml`](../trace.toml) in `proposed`
  state. (Orchestrator inline-completes.)
- [ ] **T-A5** — Gates verified: `bash scripts/verify_anchors.sh` PASS
  (34/34 currently; this brief is the one that MOVES the SHAs);
  `scripts/spec_lint.py` no new violation categories.

## M-OD — Operator decides (Q1-Q4)

_owner: operator. Q1 (canonical config) is the load-bearing one._

- [ ] **T-OD1** — Q1 canonical config. Default: (b) medium (30..=80 ms
  / 8 bps).
- [ ] **T-OD2** — Q2 retire-or-keep OLD noop anchors. Default: (a)
  keep as noop-baseline namespace.
- [ ] **T-OD3** — Q3 strategy retirement on K1 surprise. Default: (b)
  flag per scenario for operator review.
- [ ] **T-OD4** — Q4 cross-feature re-check budget. Default: (a)
  re-run all overlay e2e tests under canonical config.

## M-T1 — Architect

_owner: architect (post-operator-decide)._

- [ ] **T-AR-1** — Lock the canonical config exact values per Q1
  resolution.
- [ ] **T-AR-2** — ADR-0043 amendment OR new ADR-0045 documenting the
  canonical-config decision + the noop-baseline namespace strategy
  (per Q2 resolution).
- [ ] **T-AR-3** — Anchor-migration plan: name the new namespace pin,
  define the file-by-file rewrite contract for `spec/anchors.toml`.
- [ ] **T-AR-4** — Cross-feature re-check inventory (Q4): enumerate
  every overlay/sizing-modifier whose e2e test needs re-running. At
  minimum: vol_targeting_overlay_end_to_end + vol_killswitch_overlay
  _end_to_end + tcn_overlay (if applicable).
- [ ] **T-AR-5** — Frontmatter flip `owner: architect → developer`;
  trace.toml `arch` column populated.

## M-DEV — Developer execution (4 waves)

_owner: developer. Wave-parallelizable per architect's M-T1 lock._

### Wave A — Re-run 34 backtests under canonical config (~2-3d)

- [ ] **T-D-N1** — Apply canonical `LatencySlippageSimConfig` (per Q1
  lock) to the 34 anchored scenarios. Re-emit each report under
  the new namespace pin.
- [ ] **T-D-N2** — Confirm each scenario completes cleanly; flag any
  that error / crash / produce nonsensical equity.

### Wave B — Anchor SHA migration in spec/anchors.toml (~0.5d)

- [ ] **T-D-N3** — Compute new body-SHA-256 for each of the 34
  re-emitted reports.
- [ ] **T-D-N4** — Rewrite `spec/anchors.toml`: 34 OLD anchors move to
  the `noop-baseline` namespace (per Q2=(a)); 34 NEW anchors under
  the canonical namespace (per Q1 lock).
- [ ] **T-D-N5** — Verify `bash scripts/verify_anchors.sh` PASSes the
  full set (34 noop-baseline + 34 canonical = 68 total OR the noop
  set retires per Q2=(b)/(c)).

### Wave C — Sharpe/drawdown/final-equity delta table (~0.5d)

- [ ] **T-D-N6** — For each of the 34 scenarios: extract `final_equity`
  / `max_drawdown` / `sharpe_ratio` from both OLD-noop and NEW-canonical
  reports. Render the delta table in
  `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-MM-DD.md`.
- [ ] **T-D-N7** — Flag scenarios where strategy alpha inverted
  (positive → negative). These are K1 surprise candidates per Q3.

### Wave D — Cross-feature e2e re-checks (~2-3d)

- [ ] **T-D-N8** — Per T-AR-4 inventory: re-run each overlay e2e
  divergence test under canonical config. Confirm the divergence
  threshold still asserts correctly (≥ 1 bp) — if not, the overlay's
  test needs threshold adjustment.
- [ ] **T-D-N9** — Update cross-feature anchored fixtures (if any
  carry SHAs) under the new namespace.

### Final

- [ ] **T-D-N10** — Tick all T-D-N rows; flip frontmatter
  `owner: developer → tester`; populate trace.toml `crates` + `tests`
  columns.

## M-FINAL — Tester verification

_owner: tester._

- [ ] **T-T-1** — `bash scripts/verify_anchors.sh` PASS against the
  new anchored set (34 canonical, plus noop-baseline if Q2=(a)).
- [ ] **T-T-2** — `cargo test --workspace --no-fail-fast` — no new
  failures vs whitelist.
- [ ] **T-T-3** — Sharpe-delta table (T-D-N6 output) reviewed for K1
  surprise; per-scenario retirement candidates surfaced to operator
  per Q3.
- [ ] **T-T-4** — Cross-feature e2e tests (Wave D) all PASS under
  canonical config.
- [ ] **T-T-5** — Author
  `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/test-final-2026-MM-DD-v5-latency-slippage-sim-v0.2.0-anchor-migration.md`.
- [ ] **T-T-6** — Trace row populated + flipped `proposed → passed`.

## M-PRESENTER — Sprint-review deck

_owner: presenter. Runs only after VERDICT → PASS._

- [ ] **T-P-1** — Author
  `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/presentations/v5-latency-slippage-sim-v0.2.0-anchor-migration-2026-MM-DD.md`.
- [ ] **T-P-2** — Lead with the Sharpe-delta-per-scenario story; the
  4-cell verdict tree from feature.md; the K1 retirement candidates
  (if any).
- [ ] **T-P-3** — Operator review. Capture verdict cell.
- [ ] **T-P-4** — On operator approval, flip feature.md frontmatter
  `status: draft → shipped`; move backlog Active → Recent.

## Notes

- **The whole point of this brief**: convert v5 v0.1.0's noop ship into
  a meaningful canonical-friction ship. Every anchored alpha number now
  represents a strategy's edge UNDER simulated friction.
- **K1 / Q3 are the operator-judgment trail**: post-Sharpe-delta-table
  review, some strategies may need to be retired or accepted-as-negative.
  The brief explicitly defers per-scenario retirement to operator review.
- **Anchor-cascade safety**: Q4=(a) re-runs all overlay e2e tests under
  the canonical config — defensive against silent cross-feature
  invariant breakage.
