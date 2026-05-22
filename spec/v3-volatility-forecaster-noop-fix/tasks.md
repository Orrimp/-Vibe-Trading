---
slug: v3-volatility-forecaster-noop-fix
status: proposed
owner: analyst
updated: 2026-05-22
priority: P0
---

# v3-volatility-forecaster-noop-fix — tasks

> Per AGENT.md, task IDs use these prefixes: **T-A** analyst,
> **T-OD** operator-decide, **T-AR** architect, **T-D-N** developer
> (Wave A then B then …), **T-T** tester, **T-P** presenter.
> Architect refines the developer waves at M-T1.

## T-A — Analyst (closed at HANDOFF)

- [x] **T-A1** — Author `feature.md` v0.1.0 with R1-R6 + K1-K3 +
  H1-H2 + Q1-Q3 + 4-cell route table.
- [x] **T-A2** — Investigate TCN overlay for the same no-op pattern.
  Finding: structurally different — TCN's dampen-to-Hold semantic
  mutates `Signal.kind` (load-bearing field the executor reads);
  no parallel bug. Documented in § Investigation findings.
- [x] **T-A3** — Enumerate affected anchors (4 rows: 3 in
  `[v3.0.0-volatility]`, 1 in `[v3.0.0-volatility-rebaseline]`).
  Three expected to change post-fix; `vol-verdict-bs1-realdata`
  audit-pending (GARCH-only path may or may not cite overlay
  equity). Architect confirms at M-T1.
- [x] **T-A4** — Add `REQ-V3-VOL-FORECASTER-NOOP-FIX-001` row to
  `spec/trace.toml` with parent `REQ-V3-VOL-FORECASTER-001`,
  state `proposed`.
- [x] **T-A5** — Append Active block to `spec/backlog.md` flagging
  P0.
- [x] **T-A6** — Amendment block to `spec/v3-volatility-forecaster/feature.md
  § Verification` noting "INVALIDATED 2026-05-22 — see
  v3-volatility-forecaster-noop-fix".
- [x] **T-A7** — Amendment block to
  `spec/v3-volatility-forecaster-rebaseline/feature.md § Verification`
  similarly.
- [x] **T-A8** — Author dev-note
  `spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`
  capturing the caveman-probe diagnostic chain.

## T-OD — Operator-decide (Q1..Q3)

Standing Autoapprove from operator's 2026-05-22 prior session
applies to the analyst-recommended defaults. Orchestrator may
auto-tick all three.

- [x] **T-OD1** — Resolve **Q1** (fix shape). Default: **(ii)**
  defaulted `Strategy::quantity_scale(&self, symbol) → f64` trait
  method (minimum blast radius). — **Resolved 2026-05-22 → (ii)** by orchestrator under operator's standing Autoapprove (K2 min-blast-radius rationale; Signal serialization in audit ledger / journal is the larger blast-radius risk under (i)).
- [x] **T-OD2** — Resolve **Q2** (anchor re-emission protocol).
  Default: **(a)** re-emit in-place under existing namespaces +
  ADR-0038 § D6 amendment subsection documenting the wiring-bug-
  fix re-emission protocol. — **Resolved 2026-05-22 → (a)** by orchestrator under standing Autoapprove (precedent-setting; ADR amendment makes the protocol explicit for any future demonstrated wiring-bug discoveries).
- [x] **T-OD3** — Resolve **Q3** (TCN overlay co-investigation scope).
  Default: **(b)** vol-target-only fix; TCN audit deferred (no
  evidence of a parallel bug per T-A2). — **Resolved 2026-05-22 → (b)** by orchestrator under standing Autoapprove (T-A2 investigation confirmed TCN overlay mutates load-bearing `Signal.kind`; no parallel bug; scope stays tight).

## T-AR — Architect (M-T1)

Stubs — architect fills file:line citations, wave shape, and rollback
plan at M-T1. Replace each `<…>` placeholder.

- [ ] **T-AR-1** — Lock the Q1-selected fix shape with file:line
  citations. If Q1=(ii): the new `Strategy::quantity_scale` method
  signature; the call site in the sizing pipeline (`crates/exec/?`
  or `crates/backtest/src/engine.rs` — architect identifies); the
  vol-target overlay impl that returns the per-symbol cached scale.
  If Q1=(i): the new `trading_core::Signal.quantity_scale` field
  shape; the executor / sizing consumer that reads it; the audit /
  journal serialization implications (default-on-omit).
- [ ] **T-AR-2** — Audit `vol-verdict-bs1-realdata` to confirm
  whether its body cites overlay equity (will change post-fix) or
  only GARCH internals (stays byte-identical). Record finding in
  this tasks.md.
- [ ] **T-AR-3** — Draft the ADR-0038 § D6 amendment subsection text
  for R5. ~30 lines. The 4-clause protocol: (a) enumerate affected
  anchors, (b) cite bug site with file:line, (c) include the would-
  have-caught test, (d) architect signs off on re-emission delta.
- [ ] **T-AR-4** — Re-audit `tcn_overlay_momentum.rs` for any
  adjacent assumption that breaks under the Q1 fix (e.g. if Q1=(i)
  Signal-field change, does TCN overlay's `Signal { kind:
  modulated_kind, ..sig }` spread correctly carry the new field?).
  Q3=(b) default applies unless architect finds something.
- [ ] **T-AR-5** — Decompose the developer waves (T-D-N1..T-D-Nx)
  per the locked fix shape. Wave A: code fix + tests. Wave B:
  anchor re-emission + ADR amendment. Worst-case estimate <2 days.
- [ ] **T-AR-6** — Verify that R2's proposed end-to-end equity-
  divergence test would FAIL under pre-fix code (run it once before
  the fix lands; capture the failure as evidence the test is
  meaningful).
- [ ] **T-AR-7** — Frontmatter flip: `status: proposed → in-progress`,
  `owner: analyst → architect → developer`.

## T-D-N — Developer (Waves stubbed; architect refines at T-AR-5)

### Wave A — Wire-up fix + tests

- [ ] **T-D-N1** — Implement the Q1-locked fix at the strategy → executor
  handoff.
- [ ] **T-D-N2** — Implement R6 unit test: `scale != 1.0` causes
  returned Signal (or queried `Strategy::quantity_scale`) to carry
  the computed scale.
- [ ] **T-D-N3** — Implement R6 integration test at the engine
  boundary: two fills under `compute_scale → 1.7` differ in
  quantity from two fills under `scale = 1.0`.
- [ ] **T-D-N4** — Implement R2 end-to-end test:
  `vol_targeting_overlay::overlay_changes_equity_vs_untargeted_baseline`.
  Synthetic-or-fixture data stream with a rigged sigma_hat sequence.
  Assert equity divergence ≥ 1 bp + trade-count ≈ identical.
- [ ] **T-D-N5** — Run `cargo test --workspace --features candle`;
  confirm green. Run `cargo clippy --workspace --features candle -D
  warnings`; confirm green.

### Wave B — Anchor re-emission + ADR amendment

- [ ] **T-D-N6** — Re-run `top10-2023-fy-vol-target-overlay-realdata`
  backtest; emit new report; capture new body-SHA-256. Determinism:
  2-run byte-identity per R11.9 / R11.10 carry-forward.
- [ ] **T-D-N7** — Re-run `sharpe-comparison-vol-target-bs1-realdata`;
  emit new report; capture new body-SHA-256. Re-evaluate T-classifier
  on the new net_delta.
- [ ] **T-D-N8** — Re-run `sharpe-comparison-vol-target-bs1-realbaseline`
  (rebaseline-namespace anchor); emit new report; capture new
  body-SHA-256.
- [ ] **T-D-N9** — (Conditional on T-AR-2 finding) Re-run
  `vol-verdict-bs1-realdata` if its body cites overlay equity;
  otherwise leave byte-identical.
- [ ] **T-D-N10** — Update `spec/anchors.toml` with the new SHAs
  under existing namespace blocks (Q2=(a) default). Add a comment
  block referencing this feature and the dev-note.
- [ ] **T-D-N11** — Land the ADR-0038 § D6 amendment subsection
  (T-AR-3 text). Spec-update via the skill.
- [ ] **T-D-N12** — Run `scripts/verify_anchors.sh`; confirm
  ANCHORS PASS (34 / 34) with 3-4 fresh + 30-31 unchanged.

## T-T — Tester (M-FINAL)

- [ ] **T-T1** — cargo fmt --check PASS; cargo clippy --workspace
  --features candle -D warnings PASS; cargo test --workspace --lib
  --features candle PASS.
- [ ] **T-T2** — Re-run `scripts/verify_anchors.sh`; confirm
  ANCHORS PASS (34 / 34). Verify the affected rows show the new
  SHAs; verify the un-changed rows show their original SHAs
  (negative invariant on 30-31 rows). Capture the diff in the
  test report.
- [ ] **T-T3** — Re-run R2 + R6 regression tests; capture PASS.
  Verify (via local revert experiment) that the tests FAIL under
  pre-fix code; capture this verification in the test report as
  evidence the gate is meaningful. Authored test report at
  `spec/v3-volatility-forecaster-noop-fix/reports/test-final-<date>.md`.
- [ ] **T-T4** — Record new T-classifier + V-verdict (likely
  V3 unchanged + new T-cell) in feature.md § Verification.
- [ ] **T-T5** — Flip parent + rebaseline feature.md § Verification
  amendment block's TBD verdict-cell cross-reference to the new
  Verification block's verdict (R-O1 / R-O2 / R-O3).

## T-P — Presenter (M-PRESENTER)

- [ ] **T-P1** — Assemble
  `spec/v3-volatility-forecaster-noop-fix/presentations/v3-volatility-forecaster-noop-fix-<YYYY-MM-DD>.md`.
  Carry the 4-cell route table; recommend the (a) RETIRE-C1 (R-O1) /
  (a) RETIRE or (d) refit (R-O2) / reopen + V3 repair (R-O3) decision
  per the verdict cell. Operator ticks approval.

## Watch recipes (per MEMORY.md)

If any backtest / anchor re-emission runs >2 min, the developer
emits a copy-pasteable `watch -n 2 '<probe>'` block in the wave-
status update so the orchestrator can stream progress without
polling. Expected wall-clock per scenario: ~40s (carry-forward
from rebaseline pass).

## Changelog

- 2026-05-22 (analyst): tasks skeleton authored; T-A1..T-A8 ticked
  at HANDOFF; T-OD1..T-OD3 carry standing-Autoapprove defaults
  (Q1=(ii), Q2=(a), Q3=(b)); T-AR / T-D-N / T-T / T-P stubs left
  for architect / developer / tester / presenter refinement.
