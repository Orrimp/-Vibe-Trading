---
slug: v5-latency-slippage-sim-v0.3.0-full-path-wiring
status: draft
owner: analyst
updated: 2026-05-27
priority: P1
---

# v5 v0.3.0 full-path wiring — tasks

> Standard 6-milestone scaffold per AGENT.md. Closes the operator-
> approved v0.2.0 Ship Route (a) partial-migration follow-on commitment.

## M0 — Analyst

_owner: analyst_

- [x] **T-A1** (2026-05-27) — `feature.md` v0.1.0 authored: 6R / 6K / 4H / 5Q + R-NR + 4-cell verdict tree + cost framing.
- [x] **T-A2** (2026-05-27) — tasks.md scaffold (this file).
- [x] **T-A3** (2026-05-27) — Active row appended to [`spec/backlog.md`](../backlog.md).
- [x] **T-A4** (2026-05-27) — Trace row `REQ-V5-FULL-PATH-WIRING-001` appended at EOF of [`spec/trace.toml`](../trace.toml) in `proposed` state.
- [x] **T-A5** (2026-05-27) — `bash scripts/verify_anchors.sh` PASS (69/69). `scripts/spec_lint.py` needs Python 3.11+ (unavailable on analyst env Python 3.9.6) — orchestrator re-runs.

## M-OD — Operator decides (Q1-Q5)

_owner: operator. Q1 is LOAD-BEARING (no safe analyst default); Q2-Q5 standing-Autoapprove-eligible._

- [ ] **T-OD1** — Q1 Group A data-source: (a) revert to synthetic baseline / (b) accept real-Binance baseline as new oracle epoch. **HARD operator-decide.** See feature.md § Operator-decide table.
- [ ] **T-OD2** — Q2 Wave ordering: (a) all-then-emit *recommended* / (b) wire-and-emit per-path.
- [ ] **T-OD3** — Q3 Anchor namespace: (a) extend `v5-realdata-medium-2026-05` *recommended* / (b) new pin `v5-realdata-medium-2026-05-full`.
- [ ] **T-OD4** — Q4 t1937 resolution: (a) update constants / (b) namespace-aware resolver *recommended*.
- [ ] **T-OD5** — Q5 Cross-feature re-check: (a) re-run all overlay e2e *recommended* / (b) load-bearing 3 only / (c) defer v0.4.

## M-T1 — Architect

_owner: architect (post-operator-decide)._

- [ ] **T-AR-1** — Per-scenario plumbing audit for the 6 unwired strategy paths; reference wired path: `crates/backtest/src/scenarios/momentum.rs` lines 390, 438, 555.
- [ ] **T-AR-2** — Q1 K2 reachability check: confirm R2 route (a) (synthetic-baseline revert) is technically reachable for the 5 Group A scenarios. If unreachable, escalate to operator as forced-route-(b).
- [ ] **T-AR-3** — ADR amendment or new ADR extending ADR-0045 D2 (namespace) per Q3 and D4 (e2e inventory) per Q5.
- [ ] **T-AR-4** — R3 namespace-aware resolver design contract: Rust mirror of `verify_anchors.sh` namespace-filter walk (Q4=(b) lock).
- [ ] **T-AR-5** — Flip frontmatter `architect → developer`; populate trace.toml `arch` column with new ADR ref.

## M-DEV — Developer execution (6 waves)

_owner: developer. Wave-parallelizable where independent per AGENT.md
§ Parallelism rules._

### Wave A — R1 plumbing for 6 strategy paths (~1d)

- [ ] **T-D-N1** — Add `latency_slippage_sim: LatencySlippageSimConfig` to `SmaScenarioInput`, `PairsScenarioInput`, `TcnScenarioInput` in `crates/backtest/src/cli_types.rs`.
- [ ] **T-D-N2** — Thread the field through `scenarios/{sma_composed_run, pairs, tcn_overlay, tcn_overlay_weights, patchtst_overlay_weights, garch_vol_target_overlay, threshold_sweep}.rs::run` (mirror momentum.rs lines 390, 438, 555).
- [ ] **T-D-N3** — Wire existing v0.2.0 CLI flags (`--sim-latency-ms-{min,max}`, `--sim-slippage-bps`) in `main.rs` to populate new input fields on all 6 paths.
- [ ] **T-D-N4** — Plumbing unit test per scenario-input struct: Default = noop, non-zero config flows through.

### Wave B — Re-emit canonical reports for 32 scenarios (~1d)

- [ ] **T-D-N5** — Run 32 currently-=noop scenarios under canonical `{ 30, 80, 8 }`. Reports → `spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports/`. Group A gated on Q1 route.

### Wave C — R4 anchor SHA migration (~0.25d)

- [ ] **T-D-N6** — Compute body-SHA-256 per `scripts/hash_report.py`. Update `spec/anchors.toml` per Q3 lock.
- [ ] **T-D-N7** — `bash scripts/verify_anchors.sh` PASS (68/68 if Q3=(a) or N/N if Q3=(b)).

### Wave D — R5 Sharpe-delta table extension (~0.5d)

- [ ] **T-D-N8** — Extend v0.2.0 table to all 7 strategy paths under canonical friction; K1 surprise scan; flag flipped-alpha candidates per v0.2.0 Q3=(b) precedent. Write to `reports/sharpe-delta-table-<DATE>.md`.

### Wave E — R3 t1937 namespace-aware resolver (~0.5d)

- [ ] **T-D-N9** — Implement namespace-aware `find_backtest_report` in `crates/reports/tests/strategy_anchors_unchanged.rs` per T-AR-4 contract. Constants stay pinned to noop-baseline SHAs. Test flips GREEN.
- [ ] **T-D-N10** — Grep `crates/*/tests/*.rs` for similar hardcoded-SHA + lex-sort patterns (K4); fix or document.

### Wave F — R6 cross-feature e2e re-checks (~0.25d)

- [ ] **T-D-N11** — Re-run `vol_targeting_overlay_end_to_end.rs`, `vol_killswitch_overlay_end_to_end.rs`, `latency_slippage_sim_e2e.rs` under post-R1 canonical config. Confirm ≥ 1 bp divergence holds.

### Final

- [ ] **T-D-N12** — Tick all T-D-N rows; flip frontmatter `developer → tester`; populate trace.toml `crates` + `tests`.

## M-FINAL — Tester verification

_owner: tester._

- [ ] **T-T-1** — `bash scripts/verify_anchors.sh` PASS (68/68 or N/N per Q3).
- [ ] **T-T-2** — `cargo test --workspace --no-fail-fast` — zero NEW failures vs v0.2.0 whitelist. **CRITICAL:** `t1937_nine_strategy_anchors_unchanged` MUST FLIP TO GREEN (R3 / R-NR.4 gate).
- [ ] **T-T-3** — Sharpe-delta table reviewed for K1 surprise; retirement candidates surfaced per v0.2.0 Q3=(b) precedent.
- [ ] **T-T-4** — Wave F cross-feature e2e all PASS at ≥ 1 bp divergence.
- [ ] **T-T-5** — Author `reports/test-final-<DATE>-v5-latency-slippage-sim-v0.3.0-full-path-wiring.md`.
- [ ] **T-T-6** — Trace row populated + flipped `proposed → passed`.

## M-PRESENTER — Sprint-review deck

_owner: presenter. Runs only after VERDICT → PASS._

- [ ] **T-P-1** — Author `presentations/v5-latency-slippage-sim-v0.3.0-full-path-wiring-<DATE>.md`.
- [ ] **T-P-2** — Lead with full-coverage Sharpe-delta story (1 → 7 paths under canonical friction); 4-cell verdict tree; K1 retirement candidates; Q1 load-bearing decision in retrospect.
- [ ] **T-P-3** — Operator review. Capture verdict cell.
- [ ] **T-P-4** — On operator approval, flip feature.md `status: draft → shipped`; move backlog Active → Recent.

## Notes

- Converts v0.2.0's partial-migration ship into a full ship. Every anchored alpha now reflects realistic friction.
- Q1 is the load-bearing one: it determines Group A's re-anchor target AND whether noop-baseline SHAs for SMA/Composed remain valid friction-free oracles.
- R-NR.4 has TWO gates: (1) no NEW workspace test failures AND (2) v0.2.0-whitelisted t1937 flips to GREEN. Either regressing is a fail.
