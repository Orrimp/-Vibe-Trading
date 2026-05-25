---
slug: reflection-memory-trader-wiring
status: draft
owner: analyst
updated: 2026-05-25
---

# Tasks — reflection-memory-trader-wiring

> Stub at M0 close (analyst). Per-task decomposition deferred to
> architect M-T1 — they own the `decomp.md` ratification + Wave plan.

## M0 — analyst pass (DONE 2026-05-25)

- [x] Author `spec/reflection-memory-trader-wiring/feature.md` at
      version 0.1.0 — R1-R7 + K1-K8 + H1-H5 + Q1-Q7 + 10-item
      non-regression contract.
- [x] Append row to `spec/backlog.md ## Active`.
- [x] Open trace row `REQ-REFLECTION-TRADER-001` at `proposed` state
      in `spec/trace.toml`.
- [x] HANDOFF → architect for M-T1.

## M-T1 — architect pass

- [ ] **T-AR-1** Ratify Q1-Q7 + R1-R7 — confirm analyst-recommended
      defaults or surface deltas back to analyst.
- [ ] **T-AR-2** Inventory exact move-set: 8 source files in
      `crates/strategy/src/llm_forecaster/` + 13 integration test
      suites — list each file with target path under
      `crates/trader/src/llm_forecaster/` (or wherever Q1 lands).
- [ ] **T-AR-3** Inventory binary import sites: enumerate every
      `crates/*/src/{main,bin}.rs` and `crates/*/src/lib.rs` that
      currently imports `strategy::llm_forecaster::…` — map to
      post-move `trader::llm_forecaster::…`.
- [ ] **T-AR-4** Decide registry-arm fate (R2.3) — keep the
      `"llm_forecaster_v3"` warning arm in `strategy/registry.rs`
      vs full removal vs move to `trader/registry_arm.rs`.
- [ ] **T-AR-5** Decide gate-test tightening (R5.2 / Q4) — bundle
      `NullReflectionStore` addition into this brief's M-DEV or
      defer to follow-up.
- [ ] **T-AR-6** Decide decomp.md path-update strategy (K6) — in-place
      vs Errata vs leave-historical.
- [ ] **T-AR-7** Author `spec/reflection-memory-trader-wiring/decomp.md`
      with Wave plan: Wave A (workspace plumbing) → Wave B (file moves)
      → Wave C (import rewrites) → Wave D (gate-test tightening + R5.3
      positive assertion) → Wave E (errata + docs).
- [ ] **T-AR-8** Confirm or revise the 3-5 day cost estimate based on
      the inventories from T-AR-2 + T-AR-3.
- [ ] **T-AR-9** Append architect-T1 row to trace.toml
      `REQ-REFLECTION-TRADER-001` `arch` column.
- [ ] **T-AR-10** HANDOFF → developer for M-DEV with the wave plan.

## M-DEV — developer waves (parallel where safe)

- [ ] **T-DEV-A1** Wave A — Create `crates/trader/` skeleton:
      `Cargo.toml`, `src/lib.rs`, `src/llm_forecaster/mod.rs` empty
      stub. Add to workspace `[workspace.members]`. `cargo build -p
      trader` PASS (empty crate).
- [ ] **T-DEV-B1** Wave B — Move 8 source files from
      `crates/strategy/src/llm_forecaster/` → `crates/trader/src/
      llm_forecaster/`. `git mv` to preserve blame.
- [ ] **T-DEV-B2** Wave B — Move 13 integration test suites from
      `crates/strategy/tests/llm_forecaster_*.rs` →
      `crates/trader/tests/llm_forecaster_*.rs`. `git mv`.
- [ ] **T-DEV-B3** Wave B — Update `crates/strategy/Cargo.toml`: remove
      `reflection` dep. Update `crates/trader/Cargo.toml`: add all
      deps from R4.2.
- [ ] **T-DEV-C1** Wave C — Rewrite imports across the workspace:
      `s/strategy::llm_forecaster::/trader::llm_forecaster::/g`. Use
      the T-AR-3 inventory as ground truth. `cargo build --workspace`
      PASS.
- [ ] **T-DEV-C2** Wave C — Update `crates/strategy/src/registry.rs`
      `"llm_forecaster_v3"` arm per T-AR-4 decision.
- [ ] **T-DEV-D1** Wave D — Tighten gate-test per T-AR-5 decision (R5.2).
- [ ] **T-DEV-D2** Wave D — Add R5.3 positive-assertion gate-test
      `t1810_trader_crate_owns_reflection_retrieval`.
- [ ] **T-DEV-E1** Wave E — Update `crates/reflection/src/lib.rs`
      doc-comment line 11-18 (R7.4).
- [ ] **T-DEV-E2** Wave E — Append `## Errata` to
      `spec/v3-llm-forecaster/feature.md` per T-AR-6 decision (R7.1).
- [ ] **T-DEV-E3** Wave E — Optionally update `spec/product.md` § 3
      Trader agent footnote (R7.2) + `spec/architecture.md` module
      map (R7.3) — architect M-T1 confirms scope.
- [ ] **T-DEV-F1** Wave F — Run `cargo nextest run --workspace` and
      `scripts/verify_anchors.sh` locally; confirm 34/34 anchors +
      98/98 LLM-forecaster tests + 22+Phase F snapshots all PASS.
- [ ] **T-DEV-F2** HANDOFF → tester for M-FINAL with the green-locally
      report.

## M-FINAL — tester pass

- [ ] **T-T-1** Verify gate-test `t1809_no_strategy_crate_consumes_
      reflection_retrieval` returns to PASS (R5.1 / H1).
- [ ] **T-T-2** Verify R5.3 positive-assertion gate-test
      `t1810_trader_crate_owns_reflection_retrieval` PASS.
- [ ] **T-T-3** Run `scripts/verify_anchors.sh` — confirm `ANCHORS PASS
      (34 / 34)` (R6.1 / H2).
- [ ] **T-T-4** Run `cargo nextest run -p trader` — confirm 98 PASS
      (R6.2 / H3).
- [ ] **T-T-5** Run `cargo nextest run -p ui` — confirm 22 + Phase F
      visual snapshots + 11 layout invariants all PASS (R6.3).
- [ ] **T-T-6** Run `cargo build --workspace --bins` + cockpit-smoke
      skill — confirm no binary regressions (K4).
- [ ] **T-T-7** Run `cargo metadata` + verify `strategy → reflection`
      edge is GONE (R4.3 / H4).
- [ ] **T-T-8** Author test-final report at
      `spec/reflection-memory-trader-wiring/reports/test-final-<date>.md`
      per `rust-test` skill template.
- [ ] **T-T-9** VERDICT → PASS / REGRESSION; trace row state →
      `in-progress` (PASS) or back to `draft` (REGRESSION).
- [ ] **T-T-10** HANDOFF → presenter on PASS.

## M-PRESENTER — operator deck

- [ ] **T-P-1** Author
      `spec/reflection-memory-trader-wiring/presentations/
      reflection-memory-trader-wiring-<date>.md` per presenter contract.
- [ ] **T-P-2** Surface for operator approval: (i) gate-test red →
      green; (ii) 34/34 anchors preserved; (iii) clean workspace DAG;
      (iv) v0.1.1 deferred work (`MemoryProvider` trait when second
      consumer lands).
- [ ] **T-P-3** Operator approval → trace row state → `shipped`;
      backlog Active → Recent.

## Notes

- All M-DEV waves run sequentially within a single developer pass
  (no parallel sub-agents needed — the refactor is mechanically
  linear).
- The gate-test red on main is P0; this brief should not linger in
  Active state.
- See `feature.md § Handoff` for the architect input set.
