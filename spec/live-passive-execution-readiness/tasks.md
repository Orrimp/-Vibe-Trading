---
slug: live-passive-execution-readiness
status: draft
owner: analyst
updated: 2026-06-12
version: 0.1.0
---

# Tasks — live passive-execution readiness

> **Scoping stub.** This is the analyst-authored skeleton. The architect owns
> the real task decomposition per feature (F1/F2/F3) after answering Q1-Q6 in
> [`feature.md`](feature.md). No task here is "do work now" — they are the
> placeholders the architect/developer fill once the boundary-crossing
> amendment (Q1) is ratified. Nothing builds until then.

## Pre-condition (blocks ALL of F1/F2/F3)

- [ ] **P0 — operator ratifies the product/safety-boundary amendment (Q1).**
  `product.md` § Non-goals / § Project scope boundary currently forbid
  real-money execution. Until an amendment redefines the boundary as
  "live = passive-baseline-only, Binance spot, capped, operator-armed" AND an
  ADR codifies the arming-mechanism contract, `mode = "live"` STAYS rejected
  (config.rs:660-668) and no F-task starts. Owner: architect drafts, operator
  ratifies.

## F1 — Live exec client + real-exchange reconciliation (`REQ-LIVE-PASSIVE-EXEC-F1-001`)

- [ ] F1-T1 — `LiveExecRouter` trait + `BinanceSpotExecClient` (signed REST:
  market/limit place, status, cancel); testnet base-URL by config.
- [ ] F1-T2 — `SecretSource` trait (env-backed default + git-ignored local-file
  path); keys never logged/ledgered/committed. FAKE testnet keys in rehearsal.
- [ ] F1-T3 — account/balance/position reader (`GET /api/v3/account` signed).
- [ ] F1-T4 — exchange-filter ingestion (`LOT_SIZE`/`MIN_NOTIONAL`/`PRICE_FILTER`)
  + client-side pre-validation (under-min fails fast).
- [ ] F1-T5 — rebuild reconciler imbalance check vs REAL exchange balances
  (replace reconciler.rs:222-229 self-reference); divergence → `LedgerImbalance`.
- [ ] F1-T6 — error/retry/idempotency (client order IDs, partial fills, 429
  backoff, query-before-retry on timeout).
- [ ] F1-T7 — testnet rehearsal: full pipeline on Binance Spot Testnet (fake
  money), human-verification recipe.

## F2 — `PassiveBaseline` policy + arming mechanism (`REQ-LIVE-PASSIVE-EXEC-F2-001`)

- [ ] F2-T1 — `PassiveBaseline` policy (initial equal-weight allocation +
  monthly rebalance schedule + zero signals); shape per Q3 (schedule-driven
  recommended over per-bar `on_bar`).
- [ ] F2-T2 — `Mode::Live` variant + un-reject `mode = "live"` at parse (gated
  on P0; behind the arming guard).
- [ ] F2-T3 — the 5-condition arming guard (config mode + `.live-armed` arm-file
  + exec-side cap + secret presence + not-halted); agent can NEVER self-arm.
- [ ] F2-T4 — arming audit trail (armed/disarmed/blocked rows via the
  `strategy_events` + memo dual-write seam; operator-side-pending-ledger row).
- [ ] F2-T5 — wire the schedule-driven executor → F1 `LiveExecRouter`.
- [ ] F2-T6 — baseline-equity-divergence e2e gate (Q4 — architect rules
  APPLIES/N/A with reasoning; default APPLIES: armed equity diverges from flat
  do-nothing once allocation executes; orders actually sent).

## F3 — Canary runbook + safety drills (`REQ-LIVE-PASSIVE-EXEC-F3-001`)

- [ ] F3-T1 — `spec/runbooks/live-canary.md` — the arming ladder (testnet →
  live canary tiny-cap → scale-up) with explicit operator gates at every arm.
- [ ] F3-T2 — kill-switch-halts-live-orders drill (`.halt` cancels open +
  rejects new live orders; closes the broadcast-only caveat).
- [ ] F3-T3 — exec-side max-notional / max-position cap enforcement tests (caps
  at `LiveExecRouter`, not only strategy-side).
- [ ] F3-T4 — reconciler-vs-real-exchange divergence → halt drill.
- [ ] F3-T5 — wire `mode = "live"` as a durable-equity writer (ADR-0052 store,
  mode label `"live"`); live equity = monitoring backbone.
- [ ] F3-T6 — alerting-gap inventory + the canary cap-armed-tiny-capital
  human-verification recipe.

## Changelog

- 2026-06-12 (analyst): created scoping skeleton. P0 pre-condition (Q1 boundary
  amendment) blocks all F-tasks. F1 (exec client + real reconciliation, 7
  tasks), F2 (passive policy + arming, 6 tasks), F3 (canary runbook + drills, 6
  tasks). Architect owns real decomposition post-Q1.
