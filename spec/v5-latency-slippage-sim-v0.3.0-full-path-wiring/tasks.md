---
slug: v5-latency-slippage-sim-v0.3.0-full-path-wiring
status: in-progress
owner: operator-decide
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

_owner: operator. Per the analyst handoff K2-precedes-Q1 contract, architect M-T1 ran BEFORE this milestone to surface K2 verdict + architect leans._

**Pre-flight context for the operator (post-architect M-T1)**:
- **K2-REACHABLE-CHEAP** — route (a) for Q1 is cheap (~5 LoC CLI flag). The analyst-style "no safe default" framing has softened. See feature.md § Design T-AR-1 and ADR-0047 D1.
- **Architect Q1 lean: (a) revert to synthetic baseline.** Preserves friction-free oracle for 5/68 anchors at trivial cost. (b) still defensible if operator weights forward-looking realism higher.
- **Architect Q2-Q5 leans: concur with all analyst defaults.** Q2=(a), Q3=(a), Q4=(b), Q5=(a).

- [ ] **T-OD1** — Q1 Group A data-source: (a) revert to synthetic baseline *architect-lean post-K2* / (b) accept real-Binance baseline as new oracle epoch. Still genuinely operator judgment — value tradeoff (regression-gate semantics vs real-data realism), not feasibility-blocked. See feature.md § Operator-decide table + ADR-0047 D4 for the conditional re-emission contract.
- [ ] **T-OD2** — Q2 Wave ordering: (a) all-then-emit *analyst + architect concur* / (b) wire-and-emit per-path.
- [ ] **T-OD3** — Q3 Anchor namespace: (a) extend `v5-realdata-medium-2026-05` *analyst + architect concur* / (b) new pin `v5-realdata-medium-2026-05-full`.
- [ ] **T-OD4** — Q4 t1937 resolution: (a) update constants / (b) namespace-aware resolver *analyst + architect concur*.
- [ ] **T-OD5** — Q5 Cross-feature re-check: (a) re-run all overlay e2e *analyst + architect concur — inventory confirmed unchanged at 3 + 1 meta* / (b) load-bearing 3 only / (c) defer v0.4.

## M-T1 — Architect

_owner: architect (this milestone ran BEFORE M-OD per the K2-probe-precedes-Q1 contract in the analyst handoff)._

- [x] **T-AR-1** (2026-05-27) — **K2 reachability check** for Q1 route (a). Auto-switch lives at `crates/backtest/src/main.rs:977-1020` (single `if has_parquet` block in the SMA/Composed dispatch arm). **Verdict: K2-REACHABLE-CHEAP** — ~5 LoC CLI flag `--force-synthetic-bars` makes route (a) cheap and reachable. No refactor needed. See § Design T-AR-1 + ADR-0047 D1.
- [x] **T-AR-2** (2026-05-27) — Per-scenario plumbing audit for the 6 unwired strategy paths. See § Design T-AR-2 table; ~77 LoC total (~42 production + ~30 plumbing tests + ~5 CLI flag). `sim_slippage_cost` helper lifted to `crates/backtest/src/scenarios/sim.rs` per ADR-0047 D2. ThresholdSweep deferred (no equity surface); GarchVolOverlay confirmed IN scope (has equity surface).
- [x] **T-AR-3** (2026-05-27) — **ADR-0047** authored ([`spec/architecture/adr/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md`](../architecture/adr/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md)) covering D1-D6. Q3 default-confirmed (extend `v5-realdata-medium-2026-05`), Q5 default-confirmed (re-run all 3 e2e — inventory unchanged).
- [x] **T-AR-4** (2026-05-27) — R3 namespace-aware resolver contract locked in ADR-0047 D3. Pattern mirrors `scripts/verify_anchors.sh:63-110`. Existing `STRATEGY_ANCHORS` table stays pinned to noop SHAs; new `CANONICAL_STRATEGY_ANCHORS` table added at Wave C close. Test fans out to both with `Namespace::Noop` and `Namespace::Canonical`.
- [x] **T-AR-5** (2026-05-27) — Frontmatter flipped `analyst → operator-decide` (NOT `→ developer` — Q1 is still genuinely operator judgment; the K2 verdict made route (a) cheap but didn't pre-decide the value tradeoff). Trace.toml `arch` column updated with ADR-0047 ref at orchestrator close.

## M-DEV — Developer execution (6 waves)

_owner: developer. Wave-parallelizable where independent per AGENT.md
§ Parallelism rules._

### Wave A — R1 plumbing for 6 strategy paths (~1d, ~77 LoC per architect M-T1)

**Wave A is parallelizable across paths #1, #2, and #3-#6-as-group per AGENT.md § Parallelism rules — the three branches touch independent struct/run-fn pairs.**

- [ ] **T-D-N1a** — Lift `sim_slippage_cost` from `crates/backtest/src/scenarios/momentum.rs:551` to a new module `crates/backtest/src/scenarios/sim.rs` (behaviour-preserving move; anchor-additive per ADR-0038 § D6.a). Add `pub mod sim;` to the scenarios module. Replace momentum's private fn with `use crate::scenarios::sim::sim_slippage_cost;`. Grep gate: `grep -r "fn sim_slippage_cost" crates/backtest/src` returns exactly 1 line.
- [ ] **T-D-N1b** — Add `latency_slippage_sim: LatencySlippageSimConfig` field to:
  - `SmaScenarioInput` in `crates/backtest/src/cli_types.rs:124-141`
  - `PairsScenarioInput` in `crates/backtest/src/cli_types.rs:178-188`
  - `TcnScenarioInput` in `crates/backtest/src/cli_types.rs:195-211` (auto-propagates to TcnOverlay / TcnOverlayWeights / PatchTstOverlay / GarchVolOverlay / ThresholdSweep — shared struct)
- [ ] **T-D-N2a** — Thread `latency_slippage_sim` field through `sma_composed_run::run` at `crates/backtest/src/scenarios/sma_composed_run.rs:298`. Apply `sim_slippage_cost` at the buy/sell fill-accounting boundary in the bar loop (~lines 505-540). Mirror momentum.rs lines 386-391 (Buy) and 434-439 (Sell).
- [ ] **T-D-N2b** — Thread field through `pairs::run` at `crates/backtest/src/scenarios/pairs.rs:67`. 4-symbol universe; apply `sim_slippage_cost` per fill in the pairs fill loop.
- [ ] **T-D-N2c** — Thread field through `tcn_overlay::run` at `crates/backtest/src/scenarios/tcn_overlay.rs:69`. Apply `sim_slippage_cost` per fill (mirror momentum pattern).
- [ ] **T-D-N2d** — Thread field through `tcn_overlay_weights::run` at `crates/backtest/src/scenarios/tcn_overlay_weights.rs:31`. Each has its own fill loop even though the struct is shared.
- [ ] **T-D-N2e** — Thread field through `patchtst_overlay_weights::run` at `crates/backtest/src/scenarios/patchtst_overlay_weights.rs:46`.
- [ ] **T-D-N2f** — Thread field through `garch_vol_target_overlay::run` at `crates/backtest/src/scenarios/garch_vol_target_overlay.rs:105`.
- [ ] **T-D-N2g** — `threshold_sweep::run_cell` at `crates/backtest/src/scenarios/threshold_sweep.rs:56` is **DEFERRED** per ADR-0047 D2 — analysis sweep has no equity surface. Add a comment in the struct field doc noting the deferral.
- [ ] **T-D-N3a** — **Q1 = (a) route (architect lean)**: add `--force-synthetic-bars` CLI flag (~5 LoC) to `Args` in `crates/backtest/src/main.rs:33`. Guard the Parquet auto-detect at line 977 with `!args.force_synthetic_bars && parquet_dir.exists() && …`. **Q1 = (b) route**: skip this task; auto-detect stays as-is. Final lock at M-OD close per operator Q1.
- [ ] **T-D-N3b** — Wire existing v0.2.0 CLI flags (`--sim-latency-ms-{min,max}`, `--sim-slippage-bps` at `main.rs:76-85`) to populate the new `latency_slippage_sim` field on all 6 ScenarioInput construction sites (`main.rs:1114`, `1166`, `1179`, `1268`, `1280`, `1365`, `1377`, `1460`, `1472`, `1620`). Mirror the momentum site at `main.rs:1045-1056`.
- [ ] **T-D-N4** — Plumbing unit test per scenario-input struct (3 structs × Default-is-noop + non-zero-flows-through = 6 tests). Pattern: `latency_slippage_config_tests` at `cli_types.rs:69-115`.

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
