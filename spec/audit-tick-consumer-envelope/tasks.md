---
slug: audit-tick-consumer-envelope
status: draft
owner: analyst
updated: 2026-05-20
---

# Tasks — audit-tick-consumer-envelope

> Canonical design: [ADR-0031](../architecture/adr/0031-audit-tick-consumer-envelope.md).
> Contract: [feature.md](feature.md) (R1..R7 / Q1..Q5 / K1..K6 / H1..H5).

## M0 — Analyst synthesis  [DONE]

- [x] Read ADR-0031 + the `barter-rs` reference (linked in the ADR).
- [x] Confirm no `barter-rs` Cargo dep is being introduced (shape only).
- [x] Survey existing workspace `tokio::sync::broadcast` usage
  (`crates/agent/src/bus.rs`, `crates/data/src/funding.rs`,
  `crates/ui/src/live.rs`, `crates/agent/src/kill_switch.rs`).
- [x] Enumerate in-scope `journal::*` writers (R2.5) — 8 entry points
  (`post_fill`, `post_strategy_signal`, `strategy_event` +
  delegators, `kill_switch_tripped`, `open_uptime_interval`,
  `close_uptime_interval`, plus `ForecastEmitted` whose call site
  the architect confirms).
- [x] Close Q1-Q5 with analyst-recommended defaults (Q1=1024,
  Q2=inline post-commit, Q3=hidden side-effect/opt-in at
  construction, Q4=defer UI to follow-up, Q5=pre-seed `agent_pid` +
  `run_id` at session start).
- [x] Lock R1..R7 requirements; identify K1..K6 risks; pin H1..H5
  falsifiable hypotheses.
- [x] **Acceptance gate met:** feature.md `status: draft` /
  `owner: analyst` / version 0.1.0; `REQ-AUDIT-TICK-001` exists
  in `spec/trace.toml`.

**HANDOFF → operator-decide** (5 Qs pending).
**HANDOFF → architect** once operator answers (or analyst defaults
adopted by silence).

## M-T1 — Architect decomposition

_blocked until operator decision on Q1-Q5 (default: accept analyst
recommendations if no override within review window)._

- [ ] Ratify Q1..Q5 (operator overrides or analyst defaults).
- [ ] Publish `spec/audit-tick-consumer-envelope/decomp.md`:
  - per-writer change list (R2.5) — 8 in-scope writers, others
    explicitly out.
  - `Ledger` mutation: new field
    `Option<broadcast::Sender<AuditTick<AuditEvent>>>` + new
    constructor `Ledger::open_with_tick_bus` + `with_run_id` /
    `with_pid` test helpers (Q5).
  - `crates/audit/src/tick.rs` API surface: `AuditTick<E, C>`,
    `AuditContext`, `AuditEvent` (`#[non_exhaustive]`),
    `AuditTickStream` + `into_iter_blocking()`.
  - `crates/reflection/src/audit_tick_consumer.rs` stub spec
    (observation-only at v0.1.0; gated by
    `[reflection].audit_tick_consumer_enabled`).
  - Config additions (R7) — backward-compat serde defaults.
- [ ] Update [01-data-flow.md](../architecture/01-data-flow.md)
  edge table: add `reflection → audit (via AuditTick stream)` to
  match existing `reports → audit` shape (K3).
- [ ] Flip ADR-0031 status `proposed` → `accepted`; cross-link
  this brief in ADR header.
- [ ] Advance trace row `REQ-AUDIT-TICK-001` state `proposed` →
  `accepted`; fill `arch[]`.
- [ ] **Acceptance gate:** decomp.md exists; ADR-0031 status =
  `accepted`; `trace.toml.arch[]` non-empty.

**HANDOFF → developer** (single agent — backend-only feature, no UI
surface in this brief).

## M-DEV — Developer implementation

- [ ] Implement `crates/audit/src/tick.rs` (R1 types, R3 stream,
  `#[non_exhaustive]` enum, R6 metrics).
- [ ] Mutate `crates/audit/src/ledger.rs` (R2.1, R2.2; `run_id` and
  `agent_pid` pre-seeded per Q5).
- [ ] Wire post-commit tee in the 8 in-scope `journal::*` writers
  (R2.3, R2.5). Out-of-scope writers (`post_training_*`, costs,
  funding, heartbeat, verify_balance, registry_event) untouched.
- [ ] Implement `crates/reflection/src/audit_tick_consumer.rs` stub
  (R4). Add `audit` to `crates/reflection/Cargo.toml`
  `[dependencies]`.
- [ ] Extend `config/agent.toml` schema (R7.1, R7.2) with
  backward-compat serde defaults.
- [ ] **Tests** (developer authors, tester validates):
  - `crates/audit/tests/tick_event_size.rs` (H5; size ≤ 256B).
  - `crates/audit/tests/tick_variant_coverage.rs` (K5; every
    in-scope writer emits a tick).
  - `crates/audit/tests/tick_lag_drop.rs` (H3; lag observed,
    producer not blocked).
  - `crates/audit/tests/tick_run_id.rs` (K4; per-Ledger-clone
    run_id propagates).
  - `crates/audit/tests/tick_serde_roundtrip.rs` (R1; serialize
    every variant, deserialize bit-identical).
  - `crates/reflection/tests/audit_tick_consumer_stub.rs` (R4
    end-to-end: producer writes fill → consumer counts variant).
- [ ] Optional: `crates/audit/benches/tick_send_latency.rs` (H1;
  produce numbers, don't gate).
- [ ] **Long-running watch recipe** (per user memory): if any
  `cargo test --workspace` cycle takes > 2 min, sub-agent emits
  `watch -n 5 'tail -n 40 /tmp/audit-tick-build.log'` block.
- [ ] **Acceptance gate (self-check before tester)**:
  `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
  `cargo test --workspace`, `scripts/verify_anchors.sh` 22/22 —
  all PASS locally.

**HANDOFF → tester.**

## M-FINAL — Tester sweep

- [ ] `cargo fmt --check` exit 0.
- [ ] `cargo clippy --workspace -- -D warnings` exit 0.
- [ ] `cargo test --workspace` 100% PASS.
- [ ] `scripts/verify_anchors.sh` → 22/22 PASS (R5.1 / H2 — anchor
  preservation contract; non-negotiable per CLAUDE.md).
- [ ] `cockpit-smoke` PASS 0 panics (R5.2).
- [ ] All new tests under `crates/audit/tests/tick_*.rs` and
  `crates/reflection/tests/audit_tick_consumer_stub.rs` PASS.
- [ ] Confirm `grep -rn 'barter' Cargo.toml crates/` returns 0 hits
  (R1.4 / non-regression #6).
- [ ] Author `spec/audit-tick-consumer-envelope/reports/test-final-<YYYY-MM-DD>.md`
  per `.claude/skills/rust-test/templates/test-report.md`.
- [ ] Advance trace row `REQ-AUDIT-TICK-001` state `accepted` →
  `tested`; fill `tests[]` and `anchors[]`.
- [ ] **Acceptance gate:** VERDICT line in report = `PASS`. Any
  `REGRESSION` blocks ship per CLAUDE.md non-negotiables.

**HANDOFF → presenter** (sprint-review deck for operator approval).

## Notes

- Process-tooling feature; additive over the existing audit journal.
- Pairs naturally with future Lab Trail (Phase D) and v2.6 bake-off
  briefs as consumers — they subscribe in their own briefs, not
  this one (out of scope).
- The WAL-mode latent gap noted in ADR-0034 D1 is orthogonal and
  stays in its own backlog item — this brief does not touch
  `Ledger::open` SQL setup.
- Existing `ReflectionWriter` mpsc tap (`crates/reflection/src/writer/mod.rs`)
  stays untouched; the broadcast stub is observation-only at
  v0.1.0. A future brief migrates the lesson-write path.
