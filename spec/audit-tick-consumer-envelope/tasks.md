---
slug: audit-tick-consumer-envelope
status: draft
owner: architect
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

## M-T1 — Architect decomposition  [DONE]

- [x] Ratify Q1..Q5 (operator "Autoapprove all" on 2026-05-20 →
  all five resolved to analyst defaults).
- [x] Publish [`spec/audit-tick-consumer-envelope/decomp.md`](decomp.md)
  with per-writer change list (R2.5), `Ledger` mutation spec
  (R2.1 / R2.2 / Q5 helpers), `crates/audit/src/tick.rs` API
  surface (R1 + R3), `crates/reflection/src/audit_tick_consumer.rs`
  stub spec (R4), config additions (R7),
  `ForecastEmitted` call-site pin (decomp §5A —
  `crates/forecast/src/tcn.rs:786-795` cache-hit and `:889-898`
  post-inference), anchor-preservation discipline (R5.1 / H2),
  and the ordered landing plan (decomp §10).
- [x] Update [01-data-flow.md](../architecture/01-data-flow.md)
  edge table — added `reflection → audit (via AuditTick stream)`
  and `forecast → audit (audit-tick feature-gated)` rows (K3).
- [x] Flip ADR-0031 status `proposed` → `accepted`; added
  `refined-by` + `decomposed-by` cross-links in ADR frontmatter.
- [x] Advance trace row `REQ-AUDIT-TICK-001` state `proposed` →
  `accepted`; filled `arch[]` with decomp.md, ADR-0031,
  01-data-flow.md; expanded `crates[]` to include `crates/forecast`.
- [x] Owner `pending-architect` → `architect` in feature.md
  frontmatter.
- [x] **Acceptance gate met:** decomp.md exists; ADR-0031 status
  = `accepted`; `trace.toml.arch[]` has four entries;
  `crates/forecast` listed in `crates[]`.

**HANDOFF → developer** (single agent — backend-only feature, no
UI surface in this brief).

## M-DEV — Developer implementation

> Each row is one concrete change with a `file:line` pin (from
> [decomp.md](decomp.md)) and an exit test the developer runs
> locally before marking done. T-D-N rows land in numeric order
> per decomp §10 so anchors stay byte-identical at every step.
> All `cargo` invocations are run from the workspace root.

### Wave 1 — Envelope + ledger (tee dormant, anchors must stay 22/22)

- [ ] **T-D-1** — Create new module
  `crates/audit/src/tick.rs` with `AuditTick<E, C>`,
  `AuditContext`, `AuditEvent` (`#[non_exhaustive]`, 8 variants
  per decomp §1.1), `AuditTickStream::{new, next,
  into_iter_blocking}`, plus `pub(crate) fn emit(...)` and
  `pub fn emit_public(...)` helpers (decomp §2 + §5A). Declare
  the module in `crates/audit/src/lib.rs`. Bring no new external
  deps (R1.4); `metrics`, `tokio`, `serde`, `uuid`, `time`,
  `smol_str`, `rust_decimal` are already in the workspace.
  Exit: `cargo check -p audit` PASS.
- [ ] **T-D-2** — Mutate `crates/audit/src/ledger.rs`:
  add `tick_bus: Option<TickBus>` field (decomp §2),
  `pub(crate) struct TickBus { sender, run_id, agent_pid }`,
  preserve existing `open`/`in_memory`/`pool` bit-identical,
  add `open_with_tick_bus(db_path, capacity)`,
  `with_run_id(uuid)`, and `#[cfg(any(test, feature =
  "test-support"))] with_pid(pid)`. No call site invokes the
  new constructor yet — default boot path stays `Ledger::open`.
  Exit: `cargo check -p audit` PASS;
  `scripts/verify_anchors.sh` → 22/22 (tee dormant by default).
- [ ] **T-D-3** — Add a rustdoc banner at the head of
  `crates/audit/src/journal.rs` (above line 1, in the `//!`
  block) stating the **tee opt-in convention** per decomp §3
  K5-mitigation block. Plain markdown comment; no behaviour
  change. Exit: `cargo doc -p audit --no-deps` PASS.

### Wave 2 — Per-writer tee (still default-disabled; anchors 22/22)

For each row in decomp §3 the developer inserts **one** call:
`crate::tick::emit(ledger, AuditEvent::…);` after the writer's
final commit (or after the single-shot `execute`). The
`tick_bus = None` default branch keeps the tee dormant.

- [ ] **T-D-4** — `post_fill` tee. Pin:
  `crates/audit/src/journal.rs:65` writer body, immediately
  after `db_txn.commit().await?`. Emit
  `AuditEvent::Fill { fill: fill.clone(), fees: fill.fee.amount() }`.
  Exit: `cargo test -p audit` PASS;
  `scripts/verify_anchors.sh` → 22/22.
- [ ] **T-D-5** — `post_strategy_signal` tee. Pin:
  `crates/audit/src/journal.rs:276` writer body, immediately
  after the single `execute(&ledger.pool).await?`. Emit
  `AuditEvent::StrategySignal { strategy_id: signal.strategy_id.clone(), signal: signal.clone() }`.
  Skip the emit when the early-return `Ok(SmolStr::default())`
  Hold-signal branch fires (no SQL row → no tick). Exit:
  `cargo test -p audit` PASS; `scripts/verify_anchors.sh` →
  22/22.
- [ ] **T-D-6** — `kill_switch_tripped` tee. Pin:
  `crates/audit/src/journal.rs:775` writer body, immediately
  after `db_txn.commit().await?` (the commit at line 863). Emit
  `AuditEvent::KillSwitchTripped { reason: SmolStr::new(reason) }`.
  Exit: `cargo test -p audit` PASS;
  `scripts/verify_anchors.sh` → 22/22.
- [ ] **T-D-7** — `strategy_event` tee. Pin:
  `crates/audit/src/journal.rs:1335` writer body, immediately
  after the single `execute(&ledger.pool).await?` at line 1378.
  Emit
  `AuditEvent::StrategyEvent { kind: SmolStr::new(write.kind), payload_json: write.error_summary.unwrap_or("").to_string() }`.
  This single tee covers the four delegating writers
  (`rebalance_rejected`, `mean_reversion_stop`, `feed_reconnect`,
  `pair_short_observation`) — do NOT add tees to those. Exit:
  `cargo test -p audit` PASS; `scripts/verify_anchors.sh` →
  22/22.
- [ ] **T-D-8** — `open_uptime_interval` tee. Pin:
  `crates/audit/src/journal.rs:1621` writer body, immediately
  after the single `execute(&ledger.pool).await?`. Emit
  `AuditEvent::UptimeIntervalOpened { run_id: ledger.tick_bus.as_ref().map(|b| b.run_id).unwrap_or(Uuid::nil()) }`.
  Exit: `cargo test -p audit` PASS;
  `scripts/verify_anchors.sh` → 22/22.
- [ ] **T-D-9** — `close_uptime_interval` tee. Pin:
  `crates/audit/src/journal.rs:1674` writer body. Per
  decomp §3 row-10 note, add one
  `SELECT started_at FROM agent_uptime WHERE boot_id = ?`
  **before** the existing UPDATE; compute
  `duration_s = (now_utc - started_at).whole_seconds().max(0) as u64`
  defaulting to `0` if the row is absent. After the UPDATE
  emit
  `AuditEvent::UptimeIntervalClosed { run_id: ledger.tick_bus.as_ref().map(|b| b.run_id).unwrap_or(Uuid::nil()), duration_s }`.
  Exit: `cargo test -p audit` PASS;
  `scripts/verify_anchors.sh` → 22/22.

### Wave 3 — Config + agent bootstrap switch

- [ ] **T-D-10** — Extend `crates/agent/src/config.rs` with
  `AuditConfig { tick_bus_capacity: usize }` (default `1024`
  via `#[serde(default)]`) and add `audit_tick_consumer_enabled:
  bool` (default `false`) to `ReflectionConfig` — exact field
  shapes per decomp §6. Wire both into the top-level config
  struct. Update `config/agent.toml` with the two new sections.
  Exit: `cargo check -p agent` PASS;
  `cargo test -p agent --tests config` PASS.
- [ ] **T-D-11** — Switch agent + cockpit bootstrap to use
  `Ledger::open_with_tick_bus(path, cfg.audit.tick_bus_capacity)`
  when `tick_bus_capacity > 0`; fall back to `Ledger::open(path)`
  when `0`. Wiring sites: `crates/agent/src/main.rs:166` (uptime
  open path; replace `Ledger::open` call upstream of it) and
  `crates/ui/src/bin/cockpit_live.rs:332` (cockpit live boot).
  Default config keeps `tick_bus_capacity = 1024` → tee on by
  default in normal builds. Exit:
  `cargo check --workspace --all-features` PASS;
  `scripts/verify_anchors.sh` → 22/22 with default config
  (decomp §10 step 3 — if anchors drift here, escalate to
  architect rather than relock).

### Wave 4 — Forecast edge (gated, per decomp §5A)

- [ ] **T-D-12** — Add to
  `crates/forecast/Cargo.toml`:
  `audit = { path = "../audit", optional = true }`,
  and feature `audit-tick = ["dep:audit"]`. Default features
  list stays empty. Exit: `cargo check -p forecast` PASS;
  `cargo check -p forecast --features candle,audit-tick` PASS.
- [ ] **T-D-13** — Add `#[cfg(feature = "audit-tick")]
  pub(crate) ledger: Option<audit::Ledger>` to `TcnForecaster`
  at `crates/forecast/src/tcn.rs:420`. Add a builder
  `pub fn with_ledger(mut self, ledger: audit::Ledger) -> Self`.
  At `crates/forecast/src/tcn.rs:786-795` (cache-hit) and
  `crates/forecast/src/tcn.rs:889-898` (post-inference) insert
  the `#[cfg(feature = "audit-tick")] if let Some(l) =
  self.ledger.as_ref() { audit::tick::emit_public(l,
  audit::tick::AuditEvent::ForecastEmitted { overlay:
  overlay.clone(), cache_hit: <true|false> }); }` block as the
  *last* statement before the return / fall-through (decomp
  §5A). The existing `tracing::info!` lines stay byte-identical.
  Exit: `cargo check -p forecast --features
  candle,audit-tick` PASS.
- [ ] **T-D-14** — Wire `with_ledger(...)` from the agent /
  cockpit bootstrap (the same site as T-D-11) so live builds
  thread the `Ledger` into `TcnForecaster` when the overlay
  strategy is selected. Enable the `audit-tick` feature on
  the `forecast` dep from the bins that build the agent
  runtime (`trading` bin and `cockpit_live`), NOT from the
  training bins (`train_tcn`, `forecast_distribution`). Exit:
  `cargo build --workspace --all-features` PASS;
  `scripts/verify_anchors.sh` → 22/22 (anchors are deterministic
  on row bytes; ForecastEmitted is in-memory only).

### Wave 5 — Reflection stub

- [ ] **T-D-15** — Create
  `crates/reflection/src/audit_tick_consumer.rs` per decomp
  §1.2 with `ReflectionAuditTickConsumer::{new, run}`. Declare
  the module in `crates/reflection/src/lib.rs`. The
  `[dependencies] audit = { path = "../audit" }` line in
  `crates/reflection/Cargo.toml` already exists; no Cargo
  mutation needed. Exit: `cargo check -p reflection` PASS.
- [ ] **T-D-16** — Spawn the stub from agent bootstrap when
  `cfg.reflection.audit_tick_consumer_enabled = true`. Default
  remains `false` → stub never runs in production builds. Use
  the broadcast sender returned from
  `Ledger::open_with_tick_bus(...)` (T-D-11) and call
  `.subscribe()` once. Spawn via `tokio::spawn`. Exit:
  `cargo build --workspace` PASS;
  `cockpit-smoke` PASS 0 panics with default config.

### Wave 6 — Test surface (decomp §7)

- [ ] **T-D-17** — `crates/audit/tests/tick_event_size.rs`
  with `static_assertions::const_assert!(std::mem::size_of::<
  audit::tick::AuditEvent>() <= 256)` (H5). If the assertion
  fails, the developer boxes the offending variant (likely
  `Fill { fill: Box<Fill>, fees: Decimal }`) and re-runs. Test
  cmd: `cargo test -p audit --test tick_event_size`.
- [ ] **T-D-18** — `crates/audit/tests/tick_variant_coverage.rs`
  (K5 / R2.5): for each non-delegating writer in decomp §3 rows
  1, 2, 3, 4, 9, 10, the test wires a 64-capacity
  `Ledger::open_with_tick_bus(":memory:", 64)`, calls
  `.subscribe()`, drives the writer once with synthetic
  arguments, and asserts the next tick on the subscriber
  carries the expected variant + non-default
  `context.run_id` (when `with_run_id` was set). Delegating
  writers (`feed_reconnect`, `rebalance_rejected`,
  `mean_reversion_stop`, `pair_short_observation`) covered via
  the `StrategyEvent { kind = … }` shape. Test cmd:
  `cargo test -p audit --test tick_variant_coverage`.
- [ ] **T-D-19** — `crates/audit/tests/tick_lag_drop.rs`
  (H3 / K1): opens `Ledger::open_with_tick_bus(":memory:", 8)`,
  spawns a consumer that `sleep`s 10ms between
  `stream.next()` calls, drives `post_fill` 32 times in a
  tight loop with `Instant::now()` measurement around each
  call, asserts (a) consumer observes at least one
  `Lagged(_)` (visible as a `audit_tick_lagged_total` counter
  increment ≥ 1) AND (b) producer per-send p99 ≤ 10µs. Test
  cmd: `cargo test -p audit --test tick_lag_drop --release`.
- [ ] **T-D-20** — `crates/audit/tests/tick_run_id.rs` (K4):
  opens one `Ledger::open_with_tick_bus(":memory:", 64)`,
  clones via `with_run_id(uuid_a)` and `with_run_id(uuid_b)`,
  writes one fill on each clone, asserts the two ticks on a
  single subscriber carry the two distinct uuids. Test cmd:
  `cargo test -p audit --test tick_run_id`.
- [ ] **T-D-21** — `crates/audit/tests/tick_serde_roundtrip.rs`
  (R1.1 / R1.3): construct one `AuditTick` per variant,
  `serde_json::to_string` → `from_str` → asserts bit-identical
  `Debug` rep. Catches accidental enum-field reorder under
  `#[non_exhaustive]`. Test cmd:
  `cargo test -p audit --test tick_serde_roundtrip`.
- [ ] **T-D-22** —
  `crates/reflection/tests/audit_tick_consumer_stub.rs` (R4):
  end-to-end harness — opens
  `Ledger::open_with_tick_bus(":memory:", 64)`, spawns
  `ReflectionAuditTickConsumer::run(...)`, writes one fill,
  asserts the per-variant counter
  `reflection_audit_tick_seen_total{variant="Fill"}` reaches
  1 within 100ms. Test cmd:
  `cargo test -p reflection --test audit_tick_consumer_stub`.
- [ ] **T-D-23** *(optional)* — Criterion bench
  `crates/audit/benches/tick_send_latency.rs` (H1): measures
  `Sender::send` p99 with 0, 1, 4, 16 subscribers. Numbers
  produced, not gated (decomp §7). Cmd: `cargo bench -p audit
  --bench tick_send_latency`.

### Wave 7 — Self-check gate

- [ ] **T-D-24** — Run the M-DEV self-check (decomp §10 step 6):
  `cargo fmt --check` → exit 0;
  `cargo clippy --workspace --all-features -- -D warnings` →
  exit 0; `cargo test --workspace` → 100% PASS;
  `scripts/verify_anchors.sh` → 22/22 PASS. If any anchor
  drifts, escalate to architect (do NOT relock).
- [ ] **T-D-25** — Long-running watch recipe (per user memory):
  if T-D-24 cargo test cycle exceeds 2 min, emit
  `watch -n 5 'tail -n 40 /tmp/audit-tick-build.log'` block in
  the handoff message.

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
