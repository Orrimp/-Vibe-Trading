---
slug: live-exec-client-binance-spot
status: draft
owner: analyst
updated: 2026-06-12
version: 0.1.0
---

# Tasks — F1 Binance Spot live execution client + real-exchange reconciliation

> **Analyst seed decomposition (2026-06-12), inherited from the umbrella
> [tasks.md § F1](../live-passive-execution-readiness/tasks.md) (F1-T1..F1-T7) and
> sharpened to this folder's R/AC.** The architect owns the real M-T1 decomposition
> (resolve AQ-1..AQ-6, lock crate/file boundaries). **TESTNET-ONLY, effort L.**
> **NOTHING here un-rejects `mode = "live"`** (that is F2, atomic with the arming
> guard per ADR-0054 § D5) — `config.rs:660-668` + `t12_mode_live_is_rejected` stay
> green through F1.
>
> Design: [feature.md § Requirements / § Acceptance](feature.md);
> [ADR-0054](../architecture/adr/0054-mode-live-boundary.md);
> [umbrella § Architecture A1/A2/A6](../live-passive-execution-readiness/feature.md).

## Gate position

```
P0 (ADR-0054 § D7 ratified — CLEARED 2026-06-12)
     ▼
F1  THIS FEATURE — live exec client + real-exchange reconciliation  (TESTNET-ONLY)
     │  [GATE TO F2: testnet rehearsal passes — F1-T7 / AC-13 recipe green]
     ▼
F2  PassiveBaseline policy + 5-condition arming guard  (composes F1's cap mechanism)
```

- **Every safety-critical task names its adversarial AC** — the tester gates on those.
- **F1 builds the exec-side-cap mechanism (F1-T8) but decides nothing "armed"** — the
  5-condition guard is F2-T3 (AQ-4 fixes the seam).

## Tasks

- [ ] **F1-T1 — `LiveExecRouter` trait + `BinanceSpotExecClient` (MARKET / status /
  cancel) + signer + clock-skew.** In `crates/exec`, separate from the read-only
  `BinanceFeed`; ONE client, base-URL + keys injected (Q2 = a; never the
  `binance.rs:128-133` hard-coded-mainnet anti-pattern). HMAC-SHA256 signer (pure fn)
  + `X-MBX-APIKEY` + `recvWindow`; clock-skew sync to `GET /api/v3/time`, `-1021` →
  `HaltReason::ClockSkew`.
  _Acceptance (AC-1, AC-6): signer reproduces a fixed `(query, secret)` vector; client
  behind the trait so a fake satisfies it; testnet base-URL only; no key in `Debug`._
  _Gate (AC-12): NO real-exchange call in any test._

- [ ] **F1-T2 — `SecretSource` trait (env default + git-ignored local file) +
  `SecretString` redaction.** Q6 = a. `EnvSecretSource` (`BINANCE_API_KEY`/`_SECRET`)
  + `LocalFileSecretSource` (`config/agent.toml.local` precedent). The safe path is
  the only ingress — no code API takes a literal key. (Placement per AQ-5, default
  `crates/agent::secret`.)
  _Adversarial (AC-2): `secret_never_logged_or_serialized` — `Debug`/`Display` emit
  `<redacted>`; tracing + serde round-trips never leak the value; fixtures use
  obviously-fake placeholders only._
  _Adversarial (AC-3): absent secret ⇒ `Err(SecretError::Missing)`, fails closed —
  never a default key, never a silent unauthenticated request._
  _Gate (ADR-0054 invariant i): no key material in any fixture._

- [ ] **F1-T3 — `AccountReader` trait (`GET /api/v3/account`, signed).** Real balances
  (free+locked) parsed as `Decimal`. `None` in research/paper, `Some` only in live.
  _Acceptance (AC-4): behind a trait, faked / recorded-JSON in tests; balances
  `Decimal`._

- [ ] **F1-T4 — exchange-filter ingestion + client-side pre-validation.**
  `LOT_SIZE`/`MIN_NOTIONAL`/`PRICE_FILTER` from `GET /api/v3/exchangeInfo` into a typed
  `ExchangeFilters` (`Decimal`); round qty to `stepSize`, validate `minQty`/
  `minNotional` BEFORE submit. Freshness per AQ-2 (default TTL-cached + force-refresh
  on filter-reject). Reuse the `binance.rs:226-242` parse shape.
  _Adversarial (AC-5): `under_min_notional_fails_fast` + `bad_lot_step_rejected` — a
  bad order returns a typed `ExecError` and the faked transport records ZERO requests._
  _Gate (ADR-0054 invariant ii): all rounding `Decimal`, never `f64`._

- [ ] **F1-T5 — real-exchange reconciliation loop.** Replace `reconciler.rs:222-229`
  self-reference with a compare vs `AccountReader` balances/positions; divergence >
  `[live].reconcile_tolerance_usdt` ⇒ `HaltReason::LedgerImbalance` + audit row.
  Tolerance + debounce semantics per AQ-1 (default per-asset `Decimal` tol +
  N=2-consecutive debounce + immediate hard-trip on an unknown exchange position).
  Reconciler holds `Option<Arc<dyn AccountReader>>`.
  _Adversarial (AC-10): `reconcile_divergence_trips_halt` — injected divergent
  `AccountReader` ⇒ kill switch trips + audit row lands._
  _Gate: paper behaviour byte-unchanged when `AccountReader = None`._

- [ ] **F1-T6 — error / retry / idempotency taxonomy.** Typed `ExecError` (transport /
  rate-limit / signature / clock-skew / filter-reject / insufficient-balance /
  unknown); `newClientOrderId` for idempotency; 429/`-1003` capped exponential backoff;
  **query-before-retry on ambiguous timeout**; partial-fill accounting; N-retry
  exhaustion → halt, never silent.
  _Adversarial (AC-7): a valid order is observably submitted exactly once with a
  `newClientOrderId` + valid signature; `OrderAck` round-trips (F1's "did it leave"
  analogue)._
  _Adversarial (AC-8): `ambiguous_timeout_queries_before_resubmit` — a timed-out order
  is status-checked, NEVER blind-resubmitted._

- [ ] **F1-T7 — testnet rehearsal recipe (GATE TO F2).** Full pipeline on
  `testnet.binance.vision` with FAKE operator-provisioned keys:
  place→status→cancel→account-read→reconcile on fake money.
  _Acceptance (AC-13): self-contained human-verification recipe (Command / Steps /
  Timing / Expected / Failure-diagnosis / Cleanup); the assistant produces it, NEVER
  runs it. GATE TO F2: rehearsal green._

- [ ] **F1-T8 — exec-side cap mechanism (F1 half of the AQ-4 seam).** A standalone
  `check_notional_cap(order_notional: Decimal, cap: Decimal) -> Result<(), ExecError>`
  + the `[live].max_notional_usdt` config-field parse (`Decimal`). F1 builds + unit-
  tests the cap arithmetic + rejection path; **F2-T3's `check_armed` composes it as
  condition (3)** — F1 decides nothing "armed" (no arm-file/mode/secret-presence read
  here).
  _Adversarial (AC-11): `exec_side_cap_rejects_over_notional` — parametrized over
  (notional, cap) incl. boundary `notional == cap` ALLOWED, `notional > cap` REJECTED;
  faked transport records zero requests for the rejected case._

## Cross-cutting gates (the tester checks these on every task)

- **AC-9 (Decimal-only):** no `f64` in any order / balance / cap / filter / tolerance /
  rounding path (ADR-0003 / invariant ii). A grep/clippy guard over F1 modules backs
  the review.
- **AC-12 (no real exchange / no real key in CI):** all transport faked or recorded-
  JSON; `BINANCE_API_KEY`/`_SECRET` unset in CI; zero mainnet (`api.binance.com`) calls,
  ever (invariant iii).
- **AC-14 (scope guard):** F1 adds NO `Mode::Live` variant, does NOT un-reject
  `mode = "live"`; `t12_mode_live_is_rejected` stays green (ADR-0054 § D5).
- **AC-15 (anchor-neutral):** no `anchors.toml` row, no anchor-SHA mutation (the live
  client is never on the hashed backtest-report path).
- **Baseline-equity-divergence gate: N/A for F1 (justified).** F1 has no sizing
  decision — it is transport. The tester records N/A with the
  [feature.md § non-negotiable 4](feature.md) justification (NOT a rubber-stamp); the
  gate APPLIES to F2 (ADR-0054 § D4).

## Notes

- **Money is `Decimal` / `Money<Usdt>` everywhere** — caps, notionals, balances,
  filter quantities, tolerances (ADR-0003 / invariant ii).
- **Every external I/O behind a trait** — `LiveExecRouter`, `AccountReader`,
  `SecretSource`; no test touches a real exchange or a real key (invariant iii).
- **The safe path is the only path for secrets** — the client constructor takes
  `&dyn SecretSource`; there is no API to pass a literal key (invariant i).

## Changelog

- 2026-06-12 (analyst): created F1 task seed (v0.1.0, draft), inherited from the
  umbrella F1-T1..F1-T7 and sharpened to this folder's R/AC. Added F1-T8 (the exec-
  side cap mechanism — the F1 half of the AQ-4 F1/F2 seam) explicitly so the cap
  arithmetic is proven in isolation before F2's guard composes it. Every safety-
  critical task names its adversarial AC (AC-2 secret-redaction, AC-3 fail-closed,
  AC-5 filter-fail-fast, AC-7 order-observably-submitted, AC-8 timeout-query-before-
  resubmit, AC-10 reconcile→halt, AC-11 exec-side-cap) + the cross-cutting gates
  (AC-9 Decimal-only, AC-12 no-real-exchange/key-in-CI, AC-14 parse-rejection-
  untouched, AC-15 anchor-neutral). Recorded the baseline-divergence gate as N/A-for-
  F1 with justification (transport, no sizing). Architect owns the M-T1 decomposition
  (resolve AQ-1..AQ-6). No code, no keys, no config, no git.

---

## Appendix — `[[req]]` row for the orchestrator to apply to `spec/trace.toml`

> **Folder-scoped: the analyst does NOT edit `spec/trace.toml` directly.** The
> orchestrator applies this row. This is the **feature-level** F1 row
> (`REQ-LIVE-EXEC-CLIENT-001`); it **cross-references** the already-existing
> **program-umbrella** row `REQ-LIVE-PASSIVE-EXEC-F1-001` (created by the analyst
> 2026-06-12, state `arch-done`, at `spec/trace.toml:2886`). The umbrella row tracks
> the F1 slice at program granularity; this row tracks the buildable feature folder.

```toml
[[req]]
id          = "REQ-LIVE-EXEC-CLIENT-001"
title       = "Binance Spot live execution client + real-exchange reconciliation (F1, TESTNET-FIRST). The execution SUBSTRATE for the operator-armed passive-baseline live path ratified by ADR-0054 (§ D7 accepted 2026-06-12): an authenticated BinanceSpotExecClient behind a LiveExecRouter trait (signed REST HMAC-SHA256: POST /api/v3/order MARKET + GET /api/v3/order status + DELETE /api/v3/order cancel; ONE client, base-URL+keys injected per Q2 — NEVER the binance.rs:128-133 hard-coded-mainnet anti-pattern), a SecretSource trait (env default + git-ignored config/agent.toml.local precedent per Q6; SecretString redacts in Debug/Display/serde; the safe path is the only ingress — no code API takes a literal key; NO key material in any fixture), an AccountReader trait (GET /api/v3/account signed; balances Decimal), exchange-filter ingestion + client-side pre-validation (LOT_SIZE/MIN_NOTIONAL/PRICE_FILTER from /api/v3/exchangeInfo, round to stepSize in Decimal, under-min fails fast NEVER reaching the network), the REAL-exchange reconciliation loop (replaces reconciler.rs:222-229's self-referential heuristic with a compare vs AccountReader truth; divergence > [live].reconcile_tolerance_usdt → HaltReason::LedgerImbalance + audit row; paper byte-unchanged when AccountReader=None), HMAC signing + clock-skew (GET /api/v3/time offset, -1021 → HaltReason::ClockSkew), the error/retry/idempotency taxonomy (newClientOrderId, partial fills, 429/-1003 capped backoff, query-before-retry on ambiguous timeout — never blind-resubmit a possibly-filled order), and the exec-side cap MECHANISM (check_notional_cap + [live].max_notional_usdt parse; F2-T3's check_armed composes it). Ships TESTNET-ONLY (testnet.binance.vision) — no mainnet path armed in F1. CLAUDE.md non-negotiables: every-external-I/O-behind-a-trait + no-secrets-in-git + Decimal-never-f64 ALL apply; the baseline-equity-divergence gate is N/A for F1 (no sizing decision — a transport/client feature; the gate APPLIES to F2 per ADR-0054 § D4). Adversarial AC matrix: secret-never-logged-or-serialized, under-min-notional-fails-fast, order-observably-submitted-once, ambiguous-timeout-queries-before-resubmit, reconcile-divergence-trips-halt, exec-side-cap-rejects-over-notional, and the load-bearing no-real-exchange/no-real-key-in-CI gate (zero mainnet calls in CI, ever; testnet-URL default). Effort L. P0 (ADR-0054 § D7) ratified 2026-06-12 — BUILD unblocked; the config.rs:660-668 mode=live parse-rejection stays in force through F1 (lifted only by F2's atomic arming guard per ADR-0054 § D5). Feature-level row; cross-references the umbrella REQ-LIVE-PASSIVE-EXEC-F1-001."
feature     = "live-exec-client-binance-spot"
product     = "spec/product.md"
arch        = ["spec/architecture/adr/0054-mode-live-boundary.md", "spec/live-passive-execution-readiness/feature.md"]   # ADR-0054 boundary + umbrella A1/A2/A6; architect adds F1 M-T1 design refs
crates      = []   # developer fills (expected: crates/exec live client + cap mechanism; crates/agent::secret + reconciler AccountReader seam)
tests       = []   # developer fills (expected: signer fixed-vector, secret_never_logged_or_serialized, under_min_notional_fails_fast, bad_lot_step_rejected, ambiguous_timeout_queries_before_resubmit, reconcile_divergence_trips_halt, exec_side_cap_rejects_over_notional)
anchors     = []   # tester fills (after PASS) — expected EMPTY (F1 is anchor-neutral by construction)
state       = "proposed"
```

> **Cross-reference note for the umbrella row.** When this row is applied, the
> orchestrator should also note on `REQ-LIVE-PASSIVE-EXEC-F1-001`
> (`spec/trace.toml:2886`) that its buildable feature folder is now
> `live-exec-client-binance-spot` / `REQ-LIVE-EXEC-CLIENT-001` (the umbrella row's
> existing title already says "Ships as feature folder live-exec-client-binance-spot"
> via the umbrella tasks.md § F1 header — this row makes the trace edge concrete).
```
