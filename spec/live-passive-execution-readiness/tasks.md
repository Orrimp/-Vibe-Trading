---
slug: live-passive-execution-readiness
status: arch-done
owner: architect
updated: 2026-06-12
version: 0.2.0
---

# Tasks — live passive-execution readiness (PROGRAM UMBRELLA)

> **Architect-owned decomposition (2026-06-12).** This folder is the **program
> umbrella**: it owns P0 (the ADR-0054 boundary ratification) and the cross-feature
> architecture. **F1, F2, F3 each ship through the standard pipeline as their OWN
> feature folder** (created by the analyst at dispatch — proposed names:
> `live-exec-client-binance-spot` (F1), `passive-baseline-arming` (F2),
> `live-canary-drills` (F3)), each with its own `feature.md` / `tasks.md` /
> `reports/` and its own analyst→architect→developer→tester loop. The F-task lists
> below are the **seed decompositions** the per-feature architect passes inherit.
> Design is in [`feature.md` § Architecture](feature.md) + [ADR-0054](../architecture/adr/0054-mode-live-boundary.md).
> **NOTHING builds until P0 ratifies — no code, no keys, no config, no git.**

## Wave structure & gates (the arming ladder)

```
P0 (ADR-0054 ratified + product.md amended)
     │  [GATE: operator ratifies the D7 amendment]
     ▼
F1  live exec client + real-exchange reconciliation   (TESTNET-ONLY)
     │  [GATE: testnet rehearsal passes — F1-T7 recipe green]
     ▼
F2  PassiveBaseline policy + 5-condition arming guard  (testnet-armed)
     │  [GATE: arming guard proven on testnet — F2-T3 matrix + F2-T6 divergence green]
     ▼
F3  canary runbook + safety drills
     │  [GATE: operator arms live canary with TINY cap — explicit operator action]
     ▼
scale-up  (SEPARATE operator ratification — not a feature)
```

- **F1 → F2 hard dependency:** F2 executes through F1's `LiveExecRouter`.
- **F3 may scaffold in parallel with F2** (runbook/drill skeletons) but **gates on
  F1 + F2** (the drills exercise F1's router + F2's guard).
- **Every safety-critical task names its adversarial test** in its acceptance line —
  the tester gates on those, not on "it compiles".

## Pre-condition — P0 (blocks ALL of F1/F2/F3)

- [ ] **P0 — operator ratifies the product/safety-boundary amendment (ADR-0054 § D7).**
  Architect has drafted [ADR-0054](../architecture/adr/0054-mode-live-boundary.md)
  (`status: proposed`) with the exact proposed `product.md` § Non-goals + § Project
  scope boundary wording. **Operator action:** ratify the D7 amendment; then the
  analyst (product.md owner) applies the two edits and ADR-0054 moves
  proposed→accepted.
  _Gate: until ratified, `mode = "live"` STAYS rejected (`config.rs:660-668`,
  `t12_mode_live_is_rejected` green) and NO F-task starts._
  _Owner: architect drafts (done); operator ratifies; analyst applies edit._

---

## F1 — Live exec client + real-exchange reconciliation (`REQ-LIVE-PASSIVE-EXEC-F1-001`) — TESTNET-ONLY, effort L

> Ships as feature folder `live-exec-client-binance-spot`. Design: [feature.md § A1/A2](feature.md).

- [ ] **F1-T1 — `LiveExecRouter` trait + `BinanceSpotExecClient`** (signed REST:
  MARKET+LIMIT place, status, cancel; ONE client, base-URL+keys injected per Q2).
  In `crates/exec`, separate from the read-only `BinanceFeed`. HMAC-SHA256 signer +
  `X-MBX-APIKEY` + `recvWindow`; clock-skew sync to `GET /api/v3/time`, `-1021` →
  `HaltReason::ClockSkew`.
  _Acceptance: signer is a pure fn unit-tested against a fixed vector; client behind
  the trait so a fake satisfies it; testnet base-URL only; no key in `Debug`.
  Gate: NO real-exchange call in any test._
- [ ] **F1-T2 — `SecretSource` trait** (env-backed default + git-ignored local-file
  path, Q6). `SecretString` redacts in `Debug`/`Display`; keys NEVER
  logged/ledgered/committed/serialized.
  _Adversarial test: `secret_never_logged_or_serialized` — assert `Debug`/`Display`
  emit `<redacted>` and a round-trip through tracing/serde never leaks the value.
  Gate (invariant i): the safe path is the only ingress — no code API takes a
  literal key._
- [ ] **F1-T3 — account/balance/position reader** (`AccountReader` trait;
  `GET /api/v3/account` signed). `None` in research/paper, `Some` only in live.
  _Acceptance: behind a trait, faked in tests; balances parsed as `Decimal`._
- [ ] **F1-T4 — exchange-filter ingestion + client-side pre-validation**
  (`LOT_SIZE`/`MIN_NOTIONAL`/`PRICE_FILTER` from `GET /api/v3/exchangeInfo`; round
  qty to `stepSize` in `Decimal`).
  _Adversarial test: `under_min_notional_fails_fast` + `bad_lot_step_rejected` —
  an under-min / bad-step order returns a typed `ExecError` and NEVER reaches the
  client. Gate (invariant ii): all rounding in `Decimal`, never `f64`._
- [ ] **F1-T5 — real-exchange reconciliation** (replace `reconciler.rs:222-229`
  self-reference with a compare vs `AccountReader` balances/positions; divergence >
  `[live].reconcile_tolerance_usdt` → `HaltReason::LedgerImbalance`).
  _Adversarial test: `reconcile_divergence_trips_halt` — inject an `AccountReader`
  whose balance diverges > tol ⇒ kill switch trips + audit row lands. Gate: paper
  behaviour unchanged (`AccountReader = None` keeps the existing heuristic)._
- [ ] **F1-T6 — error/retry/idempotency** (`newClientOrderId`; partial-fill
  accounting; 429/`-1003` capped exponential backoff; **query-before-retry on
  ambiguous timeout**).
  _Adversarial test: `ambiguous_timeout_queries_before_resubmit` — a timed-out
  order is status-checked, NEVER blind-resubmitted; N-retry exhaustion → halt, never
  silent._
- [ ] **F1-T7 — testnet rehearsal** (full pipeline on `testnet.binance.vision` with
  FAKE operator-provisioned keys).
  _Acceptance: self-contained human-verification recipe (Command / Steps / Timing /
  Expected / Failure-diagnosis / Cleanup); the assistant produces it, never runs it.
  GATE TO F2: rehearsal green._

---

## F2 — `PassiveBaseline` policy + arming mechanism (`REQ-LIVE-PASSIVE-EXEC-F2-001`) — testnet-armed, effort M-L

> Ships as feature folder `passive-baseline-arming`. Depends on F1. Design:
> [feature.md § A3/A4](feature.md).

- [ ] **F2-T1 — `PassiveBaseline` schedule-driven policy** (`RebalanceSchedule` +
  `PassiveAllocator`; equal-weight inception + monthly rebalance + ZERO signals; Q3
  new seam, NOT per-bar `on_bar`). Month boundary computed from injected
  `bar.close_ts`, never `SystemTime::now()`.
  _Acceptance: schedule unit-tested against a synthetic clock (inception fires once,
  each month boundary fires once, all else empty); sizing in `Decimal`. Gate
  (determinism): no wall-clock in the decision path._
- [ ] **F2-T2 — `Mode::Live` variant + un-reject `mode = "live"` at parse** (gated
  on P0; lands ATOMICALLY with F2-T3 per ADR-0054 § D5 — never a half-armed mode).
  _Acceptance: `mode = "live"` parses ONLY in this commit alongside the guard;
  `t12_mode_live_is_rejected` is replaced by `t12b_mode_live_requires_arming` (parses
  but the guard rejects every order until all 5 conditions hold)._
- [ ] **F2-T3 — the 5-condition arming guard** (`crates/agent::arming::check_armed`;
  order 5→1→2→4→3; cap (3) + not-halted (5) enforced EXEC-SIDE at the
  `LiveExecRouter`; agent NEVER self-arms; arm-file re-checked per call).
  _Adversarial test (THE safety gate): `arming_any_one_condition_missing_zero_orders`
  — a 5-way "drop exactly one of {halt, mode, arm-file, secret, cap}" matrix; with
  all five present an order passes, with ANY single one removed `check_armed` returns
  the correct `BlockReason` and the faked client receives ZERO orders. Plus
  `agent_cannot_create_arm_file` (no code path writes `.live-armed`)._
- [ ] **F2-T4 — arming audit trail** (`LiveArmed`/`LiveDisarmed`/`LiveOrderBlocked`
  rows via the `strategy_events` + memo dual-write seam, `kill_switch.rs:287-301`
  precedent; operator-side-pending-ledger "armed live, cap=$X, T" row).
  _Adversarial test: `arming_audit_never_contains_secret` — assert no memo/row ever
  carries a key value, only presence. Gate (invariant i)._
- [ ] **F2-T5 — wire the schedule-driven executor → F1 `LiveExecRouter`** (reusing
  ADR-0053's equity/persist monitoring half verbatim; orders driven by the schedule,
  not `on_bar`).
  _Acceptance: inception + rebalance emit MARKET orders (Q5) through `check_armed` →
  `LiveExecRouter`; equity monitoring half unchanged from ADR-0052/0053._
- [ ] **F2-T6 — baseline-equity-divergence e2e gate** (ADR-0054 § D4 — APPLIES, NOT
  N/A).
  _Adversarial test (CLAUDE.md non-negotiable): `passive_inception_diverges_from_flat_baseline`
  — armed/testnet equity DIVERGES from a flat do-nothing baseline once the inception
  allocation executes (orders actually SENT; persisted series non-constant); pattern
  ref `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`. GATE TO F3 +
  REGRESSION-blocking. Tester records APPLIES._

---

## F3 — Canary runbook + safety drills (`REQ-LIVE-PASSIVE-EXEC-F3-001`) — effort M

> Ships as feature folder `live-canary-drills`. Scaffolds in parallel with F2, gates
> on F1+F2. Design: [feature.md § A5](feature.md).

- [ ] **F3-T1 — `spec/runbooks/live-canary.md`** (the arming ladder: testnet → live
  canary tiny-cap → scale-up; explicit operator gate at every arm; arming is an
  operator-only physical action; the assistant/orchestrator NEVER executes).
  _Acceptance: each rung is a self-contained recipe; "operator arms / agent never
  self-arms / assistant never executes" stated verbatim (ADR-0054 § D3)._
- [ ] **F3-T2 — kill-switch-halts-live-orders drill** (closes the broadcast-only
  caveat: `kill_switch.rs:281-284` only broadcasts `Halted` today).
  _Adversarial test: `halt_cancels_open_and_rejects_new_live_orders` — `.halt`
  present ⇒ the `LiveExecRouter` cancels ALL open live orders (`DELETE /api/v3/order`)
  AND guard (5) rejects new ones. Prove BOTH halves against a faked router._
- [ ] **F3-T3 — exec-side cap enforcement tests** (caps at the `LiveExecRouter`, not
  only the strategy-side `risk.per_symbol_exposure_cap`; defense in depth).
  _Adversarial test: `exec_side_cap_rejects_over_notional_matrix` — parametrized over
  (order notional, cap) incl. boundary `notional == cap` allowed; every over-cap
  order rejected exec-side regardless of what the sizer asked._
- [ ] **F3-T4 — reconciler-vs-real-exchange divergence → halt drill** (the F1-T5
  mechanism, drilled end-to-end).
  _Adversarial test: `live_reconcile_divergence_halts_and_reports` — divergence > tol
  ⇒ `HaltReason::LedgerImbalance` + audit row + incident report spawn._
- [ ] **F3-T5 — wire `mode = "live"` as a durable-equity writer** (ADR-0052 store,
  mode label `"live"`; live equity = monitoring backbone; reuses ADR-0052/0053, NO
  new mint site).
  _Acceptance: ≥2 live rows ⇒ cockpit KPI `Ready`; no `anchors.toml` change._
- [ ] **F3-T6 — alerting-gap inventory + canary cap-armed tiny-capital recipe.**
  _Acceptance: inventory names what unattended live needs (halt→incident-report
  already wired; assess heartbeat-to-operator channel); the canary recipe is
  self-contained (Command / Steps / Timing / Expected / Failure-diagnosis / Cleanup);
  the assistant produces it and NEVER executes it. GATE: operator arms the live
  canary as an explicit out-of-band action._

## Notes

- **Money is `Decimal`/`Money<Usdt>` everywhere** (ADR-0003 / invariant ii) — caps,
  notionals, balances, tolerances. No `f64` in any order/cap/reconcile path.
- **Every external I/O behind a trait** (`LiveExecRouter`, `AccountReader`,
  `SecretSource`) — invariant iii; no test touches a real exchange or a real key.
- **Anchor-neutral program:** no `anchors.toml` row, no anchor-SHA mutation (the
  live path never touches a hashed backtest report body). Any anchor change needs its
  own ADR.

## Changelog

- 2026-06-12 (architect): refined the analyst skeleton → `arch-done`, version 0.1.0
  → 0.2.0. Added the wave structure + arming-ladder gates (P0 → F1 → F2 → F3 →
  scale-up) with the per-wave human gate named; structured the program as an umbrella
  with F1/F2/F3 as SEPARATE feature folders. Gave every task an explicit acceptance +
  gate; named the adversarial test for every safety-critical task — the arming
  guard's `arming_any_one_condition_missing_zero_orders` 5-way matrix, the kill-switch
  `halt_cancels_open_and_rejects_new_live_orders` drill, the exec-side
  `exec_side_cap_rejects_over_notional_matrix`, the reconciler divergence→halt drill,
  the F1 `under_min_notional_fails_fast` + `ambiguous_timeout_queries_before_resubmit`,
  the secret-redaction tests, and the F2 `passive_inception_diverges_from_flat_baseline`
  REGRESSION-blocking divergence e2e (ADR-0054 § D4 APPLIES). No code, no keys, no
  config, no git.
- 2026-06-12 (analyst): created scoping skeleton. P0 pre-condition (Q1 boundary
  amendment) blocks all F-tasks. F1 (exec client + real reconciliation, 7
  tasks), F2 (passive policy + arming, 6 tasks), F3 (canary runbook + drills, 6
  tasks). Architect owns real decomposition post-Q1.
