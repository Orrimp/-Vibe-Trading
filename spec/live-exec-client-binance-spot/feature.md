---
slug: live-exec-client-binance-spot
status: presenter-done
owner: presenter
updated: 2026-06-12
version: 0.2.0
trace: REQ-LIVE-EXEC-CLIENT-001
---

# F1 — Binance Spot live execution client + real-exchange reconciliation (TESTNET-FIRST)

> **F1 of the `live-passive-execution-readiness` program umbrella.** This is the
> execution **substrate** — the authenticated exchange transport behind a trait,
> the secret boundary, the exchange-filter pre-validation, and the real-exchange
> reconciliation loop. It builds **no policy** (that is F2) and **arms nothing**
> (the 5-condition arming guard is F2; the canary runbook is F3). Ships pointed at
> **Binance Spot TESTNET only** (`https://testnet.binance.vision`) — no mainnet
> path is armed in F1.
>
> **Normative boundary:** [ADR-0054](../architecture/adr/0054-mode-live-boundary.md)
> (accepted 2026-06-12). **Cross-feature design:**
> [umbrella feature.md § Architecture A1/A2/A6](../live-passive-execution-readiness/feature.md).
> This brief makes F1 buildable: it turns A1/A2 + the ADR invariants into testable
> R/AC and surfaces the residual decisions as architect questions.

## Why

The SHIP-PASSIVE terminal verdict (`product.md` 2026-06-08, operator-ratified) and
the now-ratified boundary exception (ADR-0054 § D7, operator-ratified 2026-06-12)
mean the product may eventually run the **passive buy-and-hold baseline** live on
Binance spot — operator-armed, capped, kill-switch-supreme. **None of the transport
to do that exists today**, by design. F1 builds exactly that transport and nothing
more, testnet-first, so every later arming step (F2 guard, F3 canary) has a real
client to gate.

The capability gap is total and evidenced (re-verified against the tree, not the
umbrella brief):

| Capability                         | Status today | Evidence (file:line)                                                                 |
|------------------------------------|--------------|--------------------------------------------------------------------------------------|
| Live exchange ORDER client         | **ABSENT**   | grep clean — no `place_order`/`new_order`/HMAC/`api_secret`/signature in `crates/` or `src/`. |
| `ExecRouter` trait                 | Stub, unused | `crates/exec/src/router.rs:19` trait; `PaperExecRouter::submit` returns `UnsupportedMode("...not yet wired (T24)")`. Paper fills come from `PaperEngine::step` directly (ADR-0053 D6), NOT via `ExecRouter`. |
| Binance market-data client         | Read-only    | `crates/data/src/binance.rs:207` — `reqwest::get` UNauthenticated; WS klines/trades only. NO order/account endpoint, NO signing. |
| Account / balance / position read  | **ABSENT**   | The only `account_balance` (`crates/audit/src/query.rs:1654`) is a LEDGER query, not an exchange call. |
| Reconciler exchange-truth source   | **ABSENT**   | `reconciler.rs:58-60` `equity() = cash + position_qty * last_mark`; imbalance check (`reconciler.rs:222-229`) compares `current_equity` to its OWN prior `last_equity` — its own comment admits "full implementation needs ledger query". |
| Secret boundary precedent          | **EXISTS**   | LLM keys live in git-ignored `config/agent.toml.local` merged at load (`config.rs:612-651`, `merge_llm_local_overlay`); committed config carries only placeholders. This is the boundary F1 extends — never a new mechanism. |

**The anti-pattern F1 must NOT repeat:** `BinanceFeed::production()`
(`binance.rs:128-133`) **hard-codes** the mainnet URLs
(`wss://stream.binance.com:9443/ws`, `https://api.binance.com`). F1's client takes
its base URL + credentials **injected** (Q2 = one client, URL+keys injected) so the
exact code path that would touch mainnet is the one every testnet rehearsal
exercises — no "tested testnet, shipped untested mainnet" gap, and no hard-coded
venue.

## Scope (held hard)

**In scope (F1):**
- `LiveExecRouter` trait + `BinanceSpotExecClient` (signed REST: place MARKET order,
  query order status, cancel order) — ONE client, base-URL + keys injected.
- `SecretSource` trait (env-backed default + git-ignored local-file path) — the safe
  path is the only path; no code API takes a literal key.
- `AccountReader` trait (`GET /api/v3/account` signed) — real balances + positions.
- Exchange-filter ingestion (`LOT_SIZE` / `MIN_NOTIONAL` / `PRICE_FILTER`) +
  client-side pre-validation in `Decimal`.
- The real-exchange reconciliation loop: compare in-process/ledger state vs
  `AccountReader` truth; divergence > tolerance → kill-switch halt.
- Clock-skew / signature handling; error / retry / idempotency taxonomy.
- Testnet rehearsal recipe (full pipeline on fake money).

**Out of scope (NOT F1):**
- The `PassiveBaseline` policy, the `RebalanceSchedule`, any sizing/allocation
  decision → **F2**.
- The `Mode::Live` enum variant + un-rejecting `mode = "live"` at parse → **F2**
  (atomic with the arming guard per ADR-0054 § D5; the parse-rejection at
  `config.rs:660-668` STAYS in force through F1).
- The 5-condition arming guard (`check_armed`), the arm-file, the arming audit
  trail → **F2**. _F1 builds the exec-side cap **mechanism** the guard will call,
  but F1 never decides "armed"._
- The kill-switch live-order-cancel drill, the canary runbook, alerting inventory →
  **F3**.
- LIMIT orders → deferred operator option (Q5 = MARKET; F1 ships MARKET only; see
  AQ-3).
- Mainnet: F1 ships testnet-only; no mainnet path is armed.

## CLAUDE.md non-negotiables that bind F1 (stated, not rubber-stamped)

Three apply hard; one is **N/A with justification**.

1. **Every external I/O behind a trait** (CLAUDE.md "Every external I/O behind a
   trait so tests can fake it"; ADR-0054 invariant iii). F1 is *almost entirely*
   external I/O, so this is the load-bearing rule of the feature. The order client
   (`LiveExecRouter`), the account reader (`AccountReader`), and the secret source
   (`SecretSource`) are **each** behind a trait; the concrete Binance + env/file
   impls are the only things that touch the network or the host. **No test ever
   touches a real exchange or a real key** — this is an AC, not a hope (AC-12).

2. **No secrets in git, ever** (CLAUDE.md "No secrets in git. Keys in env / secret
   store"; ADR-0054 invariant i). The `SecretSource` design makes the safe path the
   only path: the repo ships neither keys nor a non-placeholder config; the
   constructor takes `&dyn SecretSource` and `SecretSource::get` is the sole ingress;
   `SecretString` redacts in `Debug`/`Display`/serde; keys are NEVER logged, NEVER
   written to the audit ledger, NEVER serialized. **No key material in any test
   fixture** — tests use the Binance-testnet HMAC documentation vector format with
   obviously-fake placeholder strings (see AC-2 / § Test fixtures).

3. **Money is `Decimal` / `Money<Usdt>`, never `f64`** (CLAUDE.md coding rules;
   ADR-0003; ADR-0054 invariant ii). Every notional, balance, cap, filter quantity
   (`stepSize`, `minQty`, `minNotional`, `tickSize`), reconciliation tolerance, and
   order qty is `rust_decimal::Decimal`. The existing feed already parses filters as
   `Decimal` (`binance.rs:222-242`) — F1 reuses that discipline. No `f64` in any
   order / cap / reconcile / rounding path. P&L / balance reconciliation is
   exact-cent, no tolerance band on the equality itself (the *tolerance* in R8 is the
   divergence-to-halt threshold, a deliberate `Decimal` knob, not float slop).

4. **Baseline-equity-divergence end-to-end gate — N/A for F1 (justified, not
   stamped).** The CLAUDE.md non-negotiable requires this gate for **"every strategy
   overlay or sizing-modifier"**. **F1 ships no sizing decision** — it is a
   transport/client feature: it places the order it is handed, reads balances, and
   reconciles. There is no allocation, no weight, no rebalance, no `scale` that could
   be "computed but never applied" (the exact `v3-volatility-forecaster-noop-fix`
   failure mode the gate guards). The gate therefore has **no decision variable to
   bind** in F1. **The gate APPLIES to F2** — F2 carries the inception-allocation +
   monthly-rebalance sizing decision, and ADR-0054 § D4 + the umbrella A6 already
   record APPLIES (not N/A) there, with the e2e proof
   `passive_inception_diverges_from_flat_baseline` (pattern ref
   `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`). This split is the
   honest read: you cannot prove "the allocation moved capital" in a feature that has
   no allocation. F1's analogue of "the order actually left the process" is instead
   discharged by AC-7/AC-11 (a placed order is observably submitted through the faked
   client; an over-cap / under-filter order observably does NOT reach it) and the
   testnet rehearsal AC-13. **The architect re-confirms N/A-for-F1 / APPLIES-for-F2
   at M-T1**, per the v3-vol-overlay precedent (rule the gate explicitly, every time).

## Requirements

> R = requirement. Each maps to one or more umbrella F1-T* seed tasks and to A1/A2.
> Money is `Decimal` everywhere (R-fmt); every I/O is behind a trait (R-trait).

### R1 — `LiveExecRouter` trait + `BinanceSpotExecClient` (place MARKET / status / cancel)
A `LiveExecRouter` trait (`Send + Sync`) in `crates/exec`, extending the existing
`ExecRouter` trait neighbourhood (`router.rs:19`), with at minimum:
`place_order(&Order) -> Result<OrderAck, ExecError>`, `order_status(&OrderRef) ->
Result<OrderStatus, ExecError>`, `cancel_order(&OrderRef) -> Result<(), ExecError>`.
The concrete `BinanceSpotExecClient { base_url, http, signer, … }` implements it over
signed REST: `POST /api/v3/order` (MARKET), `GET /api/v3/order` (status),
`DELETE /api/v3/order` (cancel). It is a **separate type** from `crates/data`'s
read-only `BinanceFeed` (`binance.rs:91`) — the market-data feed stays
unauthenticated; F1 reuses the `reqwest`/connect *patterns* but shares no auth code.
The client base URL + credentials are **injected** (Q2 = a), never hard-coded (the
`production()` anti-pattern). _(F1-T1; A1.)_

### R2 — `SecretSource` trait (the safe path is the only path)
A `SecretSource: Send + Sync` trait with `get(&self, key: &str) ->
Result<SecretString, SecretError>` that returns `Err(SecretError::Missing)` when
absent — **never a default key**. Default impl `EnvSecretSource` reads
`BINANCE_API_KEY` / `BINANCE_API_SECRET` from the process env (never touches repo
disk). Second impl `LocalFileSecretSource` reads the git-ignored
`config/agent.toml.local` (the proven LLM-key precedent, `config.rs:612-651`).
`SecretString` wraps the value with a `Debug`/`Display` that prints `"<redacted>"`
and a serde impl that refuses to serialize the plaintext. The client constructor
takes `&dyn SecretSource`; there is **no API to pass a literal key** in code or
committed config. _(F1-T2; A2.)_

### R3 — `AccountReader` trait (real balances + positions, signed)
An `AccountReader: Send + Sync` trait with a method returning a real account snapshot
(`balances: BTreeMap<Asset, Decimal>` free+locked, parsed as `Decimal`) from
`GET /api/v3/account` (signed). The concrete impl is part of (or sibling to)
`BinanceSpotExecClient`, using the same signer + `SecretSource`. The reconciler holds
`Option<Arc<dyn AccountReader>>`: `None` in research/paper (behaviour unchanged — the
existing self-ref heuristic stays for paper), `Some` only in live. _(F1-T3; A1.)_

### R4 — Exchange-filter ingestion + client-side pre-validation (fail fast)
Ingest `LOT_SIZE` (`stepSize`, `minQty`), `MIN_NOTIONAL` (`minNotional`), and
`PRICE_FILTER` (`tickSize`) from `GET /api/v3/exchangeInfo` into a typed
`ExchangeFilters` (all fields `Decimal`). Every order is rounded to `stepSize` and
validated against `minQty` / `minNotional` **client-side, in `Decimal`, BEFORE
submit**. An under-min-notional, under-min-qty, or bad-step order returns a typed
`ExecError` and **NEVER reaches the network**. The read-only feed already parses this
exact filter data (`binance.rs:226-242`) — reuse the parse shape; re-fetch via the
authed client for freshness (AQ-2 covers caching/refresh cadence). _(F1-T4; A1.)_

### R5 — Request signing + clock-skew handling
HMAC-SHA256 over the canonical query string with the `X-MBX-APIKEY` header +
`timestamp` + `recvWindow`. The signer is a **pure function** `(secret, query) ->
signature`, unit-testable against a fixed vector (the Binance-doc example or a
synthetic placeholder vector — never a real key). The key is **borrowed**, never
stored in the struct's `Debug`, never logged. Clock skew: sync to `GET /api/v3/time`
(server-time offset) on client construction and on any `-1021`
(timestamp-outside-recvWindow) error; persistent skew beyond a threshold →
`HaltReason::ClockSkew` (the variant already exists, `kill_switch.rs:53`). _(F1-T1;
A1.)_

### R6 — Error / retry / idempotency taxonomy
A typed `ExecError` taxonomy distinguishing at least: transport (timeout / connect),
rate-limit (HTTP 429 / Binance `-1003`), signature/auth (`-1022` / `-2014` / `-2015`),
clock-skew (`-1021`), filter-reject (mapped from R4's client-side checks AND any
exchange-side `-1013` / `-2010`), insufficient-balance, and unknown. Retry policy:
`newClientOrderId` for idempotency on retry; rate-limit → capped exponential backoff
with a hard ceiling; **on an ambiguous timeout, query order status BEFORE any
retry** — never blind-resubmit a possibly-filled order; partial-fill accounting is
tracked; N-retry exhaustion → log + `halt`, **never silent**. _(F1-T6; A1.)_

### R7 — Real-exchange reconciliation loop (replace the self-referential heuristic)
Replace `reconciler.rs:222-229`'s self-reference (it compares `current_equity` to its
own prior `last_equity`) with a comparison of in-process/ledger balances + positions
against the **`AccountReader`-reported** truth. Divergence beyond a `Decimal`
tolerance (`[live].reconcile_tolerance_usdt`, see AQ-1) →
`kill_switch.trip(HaltReason::LedgerImbalance)` (the variant already exists,
`kill_switch.rs:52`) + an audit row. Paper/research behaviour is **unchanged** when
`AccountReader = None`. _(F1-T5; A1.)_

### R8 — Exec-side cap **mechanism** (F1 builds it; F2 wires the decision)
F1 provides the exec-side cap *check* the F2 arming guard composes: an order's
notional (`Decimal`) is compared against a `[live].max_notional_usdt` (`Decimal`)
config value at the `LiveExecRouter` boundary, with `notional == cap` allowed and
`notional > cap` rejected by a typed `ExecError`. **F1 builds and unit-tests the cap
arithmetic + the rejection path** (defense-in-depth lives exec-side per ADR-0054 § D2
condition 3 / the umbrella A4). **F1 does not decide "armed"** — it never reads the
arm-file, never checks mode, never composes the 5-condition guard; that is F2-T3. The
cap mechanism is a standalone, independently-testable gate F2 calls. _(Supports
F2-T3 / F3-T3; A1 "exec-side cap guard"; see AQ-4 for the exact F1/F2 seam.)_

### R-trait (cross-cutting) — every external I/O behind a trait
`LiveExecRouter`, `AccountReader`, `SecretSource` are each a trait with a Binance/env
concrete impl and a test fake. No F1 code outside the concrete impls touches the
network or the host secret store. _(ADR-0054 invariant iii; AC-12.)_

### R-fmt (cross-cutting) — `Decimal` everywhere
No `f64` in any order, balance, cap, filter, tolerance, or rounding path. _(ADR-0003;
ADR-0054 invariant ii; AC-9.)_

## Acceptance criteria

> AC = acceptance criterion. AC-7/AC-9/AC-10/AC-11/AC-12 are the **adversarial
> matrix** entries ADR-0054 § D2/D3 + the umbrella A5/A6 name for F1. The tester
> gates on these by name (not on "it compiles").

- **AC-1 — trait + client shape.** `LiveExecRouter` exists with place(MARKET)/status/
  cancel; `BinanceSpotExecClient` implements it; a test fake also implements it.
  Client base URL + keys are constructor-injected; there is no hard-coded venue URL
  in the client. _(R1.)_
- **AC-2 — `SecretSource` + redaction (ADVERSARIAL).** Test
  `secret_never_logged_or_serialized`: `SecretString`'s `Debug` and `Display` emit
  `"<redacted>"`, and a round-trip through `tracing` and through `serde` never emits
  the plaintext. **No real key in any fixture** — the test uses an obviously-fake
  placeholder (e.g. `"FAKE_TESTNET_KEY_DO_NOT_USE"` / the Binance-doc example secret
  `"NhqPtmdSJYdKjVHjA7PZj4Mge3R5YNiP1e3UZjInClVN65XAbvqqM6A7H5fATj0j"` documented as
  a public example, never a live secret). _(R2; ADR-0054 invariant i.)_
- **AC-3 — `Missing` fails closed.** Constructing the client / requesting keys with
  the secret absent returns `Err(SecretError::Missing)` (or a constructor error that
  fails closed) — **never a default/empty key, never a silent unauthenticated
  request**. _(R2; feeds the F2 arming condition (4).)_
- **AC-4 — account read parses `Decimal`.** `AccountReader` returns balances parsed
  as `Decimal` from a recorded `GET /api/v3/account` JSON fixture (the API-shape, no
  network); free+locked split preserved. _(R3.)_
- **AC-5 — filter pre-validation (ADVERSARIAL).** Tests `under_min_notional_fails_fast`
  + `bad_lot_step_rejected`: an order below `minNotional`, below `minQty`, or off
  `stepSize` returns a typed `ExecError` and the faked transport records **zero**
  outbound requests. All rounding is `Decimal`. _(R4; ADR-0054 invariant ii.)_
- **AC-6 — signer fixed-vector.** The HMAC-SHA256 signer is a pure fn unit-tested to
  reproduce a fixed `(query, secret) -> signature` vector exactly; the key never
  appears in any `Debug` output of the client. _(R5.)_
- **AC-7 — placed order is observably submitted (ADVERSARIAL — F1's "did it actually
  leave" analogue).** A valid in-filter order, placed through the client, is
  observably submitted exactly once to the (faked) transport with a `newClientOrderId`
  and a valid signature; the returned `OrderAck` round-trips. This is F1's analogue
  of the baseline-divergence "orders actually SENT" check (the divergence gate proper
  is F2's, per § non-negotiable 4). _(R1/R6.)_
- **AC-8 — ambiguous-timeout queries before resubmit (ADVERSARIAL).** Test
  `ambiguous_timeout_queries_before_resubmit`: a timed-out place is status-checked via
  `GET /api/v3/order` and **never blind-resubmitted**; N-retry exhaustion → halt, never
  silent. Rate-limit (429 / `-1003`) backs off with a capped ceiling. _(R6.)_
- **AC-9 — `Decimal` everywhere (ADVERSARIAL/STATIC).** No `f64` appears in any order,
  balance, cap, filter, tolerance, or rounding type or arithmetic path. A grep/clippy
  guard over the F1 modules backs the review. _(R-fmt; ADR-0003.)_
- **AC-10 — reconcile divergence trips halt (ADVERSARIAL).** Test
  `reconcile_divergence_trips_halt`: inject an `AccountReader` whose reported balance
  diverges > `reconcile_tolerance_usdt` from in-process state ⇒
  `HaltReason::LedgerImbalance` trips + the audit row lands. With `AccountReader =
  None` (paper) the existing heuristic is byte-unchanged. _(R7.)_
- **AC-11 — exec-side cap rejects over-notional (ADVERSARIAL).** Test
  `exec_side_cap_rejects_over_notional`: parametrized over (order notional, cap) incl.
  the boundary `notional == cap` **allowed** and `notional > cap` **rejected** with a
  typed `ExecError`; the faked transport records zero requests for the rejected case.
  This is the F1 cap **mechanism** the F2 guard + F3 matrix build on. _(R8.)_
- **AC-12 — no real exchange / no real key in CI (ADVERSARIAL — the load-bearing CI
  gate).** Every F1 test runs with **zero** network calls to any Binance host and
  **zero** real credentials: all transport is the faked `LiveExecRouter` /
  `AccountReader` or recorded JSON fixtures; the default test base URL is the testnet
  host string used only as data (never dialed). `BINANCE_API_KEY`/`_SECRET` are unset
  in CI and the suite passes. **Zero mainnet (`api.binance.com`) calls in CI**, ever.
  _(R-trait; ADR-0054 invariant iii; "no test ever touches a real exchange or a real
  key".)_
- **AC-13 — testnet rehearsal recipe.** A self-contained human-verification recipe
  (Command / Steps / Timing / Expected-result / Failure-diagnosis / Cleanup) the
  **operator** runs against `testnet.binance.vision` with FAKE operator-provisioned
  testnet keys to exercise the full place→status→cancel→account-read→reconcile
  pipeline on fake money. The assistant **produces** the recipe and **never runs**
  it. **GATE TO F2.** _(F1-T7; the project recipe contract.)_
- **AC-14 — parse-rejection untouched.** F1 adds NO `Mode::Live` variant and does NOT
  un-reject `mode = "live"`; `config.rs:660-668` + `t12_mode_live_is_rejected` stay
  green (ADR-0054 § D5 — the un-rejection is F2-atomic). _(Scope guard.)_
- **AC-15 — anchor-neutral.** F1 mutates NO `anchors.toml` row and NO anchor SHA — the
  live client is never on the backtest/report path (the backtest never calls it). Any
  anchor change would need its own ADR (none is needed). _(Umbrella A6; ADR-0054
  § Consequences "anchor-neutral".)_

## Test fixtures (no key material, ever)

- **HMAC signer vector (AC-6):** use the Binance API-docs public example pair
  (documented as an example, not a live credential) OR a synthetic
  `(query="symbol=BTCUSDT&side=BUY&type=MARKET&quantity=1&timestamp=…",
  secret="FAKE_TESTNET_SECRET_DO_NOT_USE")` placeholder. The expected signature is
  computed once and pinned. **No file in `crates/` or `spec/` carries a real key.**
- **`GET /api/v3/account` JSON (AC-4):** a recorded testnet-shape response with
  obviously-synthetic balances; no account identifiers, no keys.
- **`GET /api/v3/exchangeInfo` filters (AC-5):** the `LOT_SIZE`/`MIN_NOTIONAL`/
  `PRICE_FILTER` JSON for `BTCUSDT` (public, key-free data).
- **Placeholder secret strings (AC-2/AC-3):** `"FAKE_TESTNET_KEY_DO_NOT_USE"` /
  `"FAKE_TESTNET_SECRET_DO_NOT_USE"` — strings that are self-evidently non-credentials.

## Open questions for the architect

> Each carries a recommended default per house style. **AQ-1 is the riskiest** (it is
> the safety-tolerance + halt-trigger calibration on real money). Recommended tags
> follow the durable-over-quick rule (`AGENT.md § Decision framing`).

- **AQ-1 (RISKIEST) — reconciliation divergence semantics: what tolerance, over what
  quantities, and how is "divergence" debounced so a single transient mismatch
  doesn't false-halt live trading?** The reconciler must compare in-process/ledger
  state to exchange truth and halt on divergence (R7), but a too-tight tolerance
  false-halts on benign timing (an in-flight fill not yet reflected), and a too-loose
  one misses a real drift. **Recommended default (durable): (a) a per-asset `Decimal`
  tolerance `[live].reconcile_tolerance_usdt` evaluated on notional-valued balance
  deltas, with an N-consecutive-divergent-reads debounce (default N=2) before the
  `LedgerImbalance` trip — and a HARD immediate trip (no debounce) if the exchange
  reports a position the ledger has zero record of** (an unknown position is never
  benign). This is durable because it encodes the two genuinely different failure
  classes (transient timing vs structural unknown) rather than one flat number that a
  v0.2.0 cleanup brief would have to re-split. *If-budget-tightens fallback:* (b) a
  single flat `Decimal` tolerance, immediate trip, no debounce — simpler and smaller,
  but it will false-halt on the first in-flight fill and spawns a v0.2.0
  "debounce the reconciler" follow-on. **Risk if wrong:** a false-halt mid-canary
  erodes operator trust; a missed drift is a silent real-money divergence — both are
  load-bearing, which is why this is AQ-1.

- **AQ-2 — exchange-filter freshness: fetch-once-at-construction, per-order, or
  TTL-cached?** Filters change rarely but a stale `minNotional` could pass a
  client-side check the exchange then rejects. **Recommended default (durable): (c) a
  TTL-cached fetch (default TTL 1 h) via the authed client, refreshed lazily on the
  next order after expiry, with a forced refresh on any exchange-side filter-reject
  (`-1013` / `-2010`)** — this keeps the common path cheap (no per-order
  `exchangeInfo` round-trip) while self-healing on the one signal that proves the
  cache is stale. *Fallback:* (a) fetch-once-at-construction — cheapest, but a
  long-lived process drifts and needs a restart to pick up a filter change (a v0.2.0
  follow-on). (b) per-order fetch is rejected as needlessly chatty (rate-limit
  pressure on a low-turnover baseline).

- **AQ-3 — order-type surface in F1: MARKET-only, or include LIMIT plumbing behind a
  flag now?** ADR-0054 / umbrella Q5 ratified MARKET for the passive baseline.
  **Recommended default (durable for F1's scope): (a) MARKET-only in F1** — the
  `place_order` signature is shaped to *admit* a future `OrderType::Limit` (an enum
  field, not a second method) so adding LIMIT in a later feature is additive, but F1
  ships and tests MARKET only. This is the durable-yet-correct choice **because the
  durable axis here is the program-level Q5 ruling (MARKET first), not gold-plating
  F1 with an unused LIMIT path**; building LIMIT now would be scope F2/F3 hasn't asked
  for. *Fallback:* (b) build LIMIT plumbing now — rejected as premature; LIMIT is a
  named operator option deferred to "if canary slippage proves material" (F3
  inventory). _(This is the AGENT.md exception case: the cheap path (a) is Recommended
  because the architect can prove it spawns no rework — the enum-shaped signature
  makes LIMIT additive, no carve-out, no MIGRATION annotation.)_

- **AQ-4 — the F1/F2 exec-side-cap seam: where exactly does F1 stop and F2 start?**
  R8 says F1 builds the cap *mechanism* and F2 wires the *decision*. **Recommended
  default (durable): (a) F1 ships a standalone `fn check_notional_cap(order_notional:
  Decimal, cap: Decimal) -> Result<(), ExecError>` + the `[live].max_notional_usdt`
  config field parse, both unit-tested (AC-11); F2's `check_armed` (F2-T3) *calls*
  this as condition (3) alongside the other four conditions.** This is durable because
  the cap arithmetic — the part that must be exactly right on real money — is proven
  in F1 in isolation, and F2 composes proven parts rather than re-deriving the cap
  inside the guard. *Fallback:* (b) defer the entire cap to F2 — rejected because it
  bundles the cap arithmetic into the guard's adversarial matrix, making the matrix
  test prove two things at once (composition AND arithmetic) instead of one.

- **AQ-5 — crate placement of `SecretSource`: `crates/exec`, `crates/agent`, or a new
  `crates/secret`?** The umbrella A1/A2 floats `crates/agent::secret` OR
  `crates/exec::secret`. **Recommended default (durable): (a) `crates/agent::secret`**
  — the secret boundary is an agent-wide concern (the LLM-key precedent already lives
  in the agent's config load, `config.rs:612-651`), and F2's arming guard (also
  `crates/agent::arming`) reads secret *presence*, so co-locating keeps the
  agent-owns-the-secret-boundary story coherent and avoids `exec → agent` or `agent →
  exec` dependency churn. The exec client takes `&dyn SecretSource` so placement is a
  dependency-direction decision, not a coupling one. *Fallback:* (b) `crates/exec` —
  acceptable if the architect finds the agent→exec edge cleaner; (c) a new
  `crates/secret` is rejected as over-factoring for two impls.

- **AQ-6 — testnet base-URL handling in tests: a typed `Testnet`/`Mainnet` enum, or a
  raw injected `base_url: String`?** Q2 ruled ONE client with URL injected.
  **Recommended default (durable): (a) a small `ExecEndpoint { base_url, label }`
  value (or a `Network::{Testnet, Mainnet}` enum that *resolves to* a base URL) the
  client takes by injection, so the testnet/mainnet choice is an explicit,
  greppable, audit-loggable value rather than a bare string** — this makes
  "F1 ships testnet-only" enforceable (a test asserts the default endpoint is
  testnet) and makes a future mainnet arming an explicit, reviewable change, not a
  string edit. *Fallback:* (b) a raw `base_url: String` — simplest, but loses the
  "which network am I pointed at" signal the arming audit will want; only acceptable
  if the architect adds a separate network-label field.

## Assumptions (challenge these)

- A1: ADR-0054 is **accepted** (operator ratified § D7, 2026-06-12) and the
  `product.md` boundary now permits this program — so F1 may BUILD (P0 cleared). The
  `mode = "live"` parse-rejection nonetheless stays in force through F1 (lifted only
  by F2's atomic guard, ADR-0054 § D5).
- A2: Binance Spot **Testnet** (`testnet.binance.vision`) is the F1 rehearsal venue;
  the operator provisions FAKE testnet keys out-of-band. The repo never sees any key
  (testnet or mainnet).
- A3: F1 builds transport only. No policy, no arming decision, no `Mode::Live`
  variant — those are F2 (this is the umbrella's hard F1→F2 dependency direction).
- A4: The reconciler's paper/research path is preserved byte-for-byte when
  `AccountReader = None` (R7/AC-10) — F1 is additive to the existing reconciler, not a
  rewrite of its paper behaviour.
- A5: F1 is anchor-neutral (AC-15) — the live client is never on the hashed
  backtest-report path.

## Architecture

> **Owner: architect. Status: `arch-done` (2026-06-12).** This section tightens the
> umbrella [§ Architecture A1/A2/A6](../live-passive-execution-readiness/feature.md)
> from program granularity to F1's module/file boundaries and resolves AQ-1..AQ-6.
> The normative boundary + the 5-condition arming contract + the five binding
> invariants live in [ADR-0054 § D1/D2/D3](../architecture/adr/0054-mode-live-boundary.md)
> — F1 is **implementation strictly under that boundary**, not a re-decision of it.
> **No code, no keys, no config, no git in this pass** — design only; the
> `config.rs:660-668` parse-rejection stays in force through F1 (ADR-0054 § D5).
>
> **ADR decision: NO new ADR for F1.** ADR-0054 already fixes the boundary
> (D1 permitted shape), the arming contract (D2 — F1 builds the cap *mechanism* for
> condition 3 but decides nothing armed), and the five invariants (D3). F1 introduces
> **no new architecturally-significant tradeoff** that ADR-0054 does not already
> govern: the AQ resolutions below are implementation choices *within* the ratified
> boundary (a tolerance knob, a cache TTL, a crate placement), not boundary moves. The
> one place an ADR *would* be required — touching an anchor SHA in `spec/anchors.toml`
> — is explicitly **not** crossed (A6 / AC-15: F1 is anchor-neutral by construction).
> `arch` therefore cites ADR-0054 + the umbrella; **no ADR-0055 is registered.**

### Binding law (verbatim — reproduced from ADR-0054 § D3 because F1 is where it lands)

> 1. **No secrets in git, ever.** The `SecretSource` design makes **the safe path the
>    only path** — the repo ships neither keys nor a non-placeholder config; keys are
>    read from env or a git-ignored local file, and are NEVER logged, NEVER written to
>    the audit ledger, NEVER serialized, NEVER committed.
> 2. **Money is `Decimal` / `Money<Usdt>`, never `f64`** (ADR-0003). Every notional,
>    cap, balance, filter quantity, and reconciliation tolerance is `rust_decimal::Decimal`;
>    P&L/balance reconciliation is exact-cent (the R8/AQ-1 *tolerance* is the
>    divergence-to-halt knob, not float slop on the equality).
> 3. **Every external I/O is behind a trait.** `LiveExecRouter`, `AccountReader`,
>    `SecretSource` are each a trait with a Binance/env concrete impl + a test fake;
>    **no F1 test ever touches a real exchange or a real key** (AC-12).
> 4. **The operator arms; the agent never self-arms; the assistant never executes.**
>    F1 builds the exec-side cap *mechanism* (R8) but never reads the arm-file, never
>    checks mode, never composes the guard, never places a real order. The assistant
>    produces the testnet rehearsal recipe (AC-13) and **never runs it**.
> 5. **The kill switch is supreme.** F1 wires reconciliation divergence → `trip()`; a
>    tripped kill switch is terminal-disarmed (the existing sticky-halt contract). F1
>    adds the divergence *trigger*; the live-order-cancel-on-halt drill is F3.
>
> Plus the project-wide guardrails F1 honours: **zero mainnet (`api.binance.com`)
> calls anywhere in CI/tests, ever** (AC-12); **anchored reports byte-immutable**
> (AC-15 — F1 touches none).

### A1 — Module layout in `crates/exec` + the `crates/agent::secret` boundary (AQ-5)

The authenticated client is a **separate type** from `crates/data`'s read-only
`BinanceFeed` (`binance.rs:91`): F1 reuses the `reqwest` 0.12 + connect *patterns*
but **shares no auth code** and never imports the feed. Proposed file layout:

```
crates/exec/src/
├── router.rs              # EXISTING — ExecRouter + PaperExecRouter (untouched).
│                          #   F1 ADDS the `LiveExecRouter` trait here (sibling, Send+Sync).
├── live/
│   ├── mod.rs            # BinanceSpotExecClient { endpoint, http, account, filters, signer }
│   │                    #   impls LiveExecRouter + AccountReader. Network injected (A1/AQ-6).
│   ├── sign.rs          # PURE fn sign(secret: &[u8], query: &str) -> String (HMAC-SHA256→hex).
│   │                    #   Borrows the secret; never stores it; not in any Debug. (AC-6)
│   ├── clock.rs         # ServerTimeOffset — sync to GET /api/v3/time on ctor + on -1021;
│   │                    #   persistent skew > threshold → maps to HaltReason::ClockSkew. (R5)
│   ├── filters.rs       # ExchangeFilters { step_size, min_qty, min_notional, tick_size: Decimal }
│   │                    #   + round_to_step()/validate() in Decimal; TTL cache (AQ-2). (R4)
│   ├── cap.rs           # check_notional_cap(notional: Decimal, cap: Decimal) -> Result<(),ExecError>
│   │                    #   STANDALONE pure fn — the F1 half of the AQ-4 seam. (R8/AC-11)
│   ├── error.rs         # ExecError taxonomy (see A4) + Binance code → variant mapping. (R6)
│   ├── endpoint.rs      # Network::{Testnet, Mainnet} → base_url + label; DEFAULT Testnet. (AQ-6)
│   └── types.rs         # OrderAck / OrderRef / OrderStatus / AccountSnapshot (serde over JSON).
└── lib.rs               # re-exports; `pub use live::{BinanceSpotExecClient, …};`
```

- **Trait boundary the agent consumes (R1/R3).** Two new traits in `crates/exec`,
  both `Send + Sync`:

  ```rust
  #[async_trait]
  pub trait LiveExecRouter: Send + Sync {
      async fn place_order(&self, order: &Order)   -> Result<OrderAck, ExecError>;
      async fn order_status(&self, r: &OrderRef)   -> Result<OrderStatus, ExecError>;
      async fn cancel_order(&self, r: &OrderRef)   -> Result<(), ExecError>;
  }
  #[async_trait]
  pub trait AccountReader: Send + Sync {
      async fn account_snapshot(&self) -> Result<AccountSnapshot, ExecError>;
  }
  // AccountSnapshot.balances: BTreeMap<Asset, Balance{ free: Decimal, locked: Decimal }>
  ```

  `BinanceSpotExecClient` implements **both** (one signer + one `&dyn SecretSource`).
  The agent (reconciler, and later F2's guard) holds `Option<Arc<dyn AccountReader>>` /
  `Option<Arc<dyn LiveExecRouter>>` — `None` in research/paper, `Some` only in live.
  `place_order` takes `&Order` (immutable `&self`, not `&mut` like the legacy
  `ExecRouter::submit`) so one `Arc<dyn LiveExecRouter>` is shareable across tasks.

- **MARKET-only, but the signature ADMITS LIMIT additively (AQ-3 = a).** `Order`
  **already** carries `kind: OrderKind` where `OrderKind::{Market, Limit{price}}`
  (`crates/core/src/order.rs:39-42`). So the enum-shaped admission AQ-3 asks for is
  **already native** — no signature change is needed to make LIMIT additive later.
  F1's client **rejects `OrderKind::Limit` at the boundary** with
  `ExecError::UnsupportedOrderType` (a typed reject, never silently dropped) and
  ships/tests MARKET only. Adding LIMIT in a later feature is purely additive (handle
  the existing variant); no carve-out, no MIGRATION annotation, no rework — which is
  why the cheap path is the durable one here (AGENT.md § Decision-framing exception).

- **`SecretSource` lives in `crates/agent::secret` (AQ-5 = a), sharpened.** The trait
  `SecretSource` + the `SecretString` newtype + `EnvSecretSource` + `LocalFileSecretSource`
  live in a new `crates/agent/src/secret.rs`. Rationale grounded in the tree: the
  secret boundary is already an **agent** concern — the LLM-key overlay
  `merge_llm_local_overlay` lives in `crates/agent/src/config.rs:612-651`, and F2's
  arming guard (also `crates/agent`) reads secret *presence* for condition (4). The
  dependency edge **`agent → exec` already exists** (the runtime constructs exec
  routers), so the exec client taking `&dyn SecretSource` introduces **no new edge and
  no cycle**: `crates/exec` defines the consuming traits, `crates/agent` provides the
  `SecretSource` impls and constructs `BinanceSpotExecClient` passing `&dyn SecretSource`
  in. (To keep `crates/exec` from depending on `crates/agent`, the **`SecretSource`
  trait declaration is re-exported into `exec` via a thin shared definition** —
  concretely: declare `SecretSource`/`SecretString`/`SecretError` in `crates/core`
  (no deps, already the shared vocabulary crate for `Money`/`Order`), impl them in
  `crates/agent::secret`, consume `&dyn SecretSource` in `crates/exec`. This is the
  dependency-direction-clean realization of AQ-5(a) — placement of the *impls* is
  agent, placement of the *trait* is core so both exec and agent see it without a
  cycle. Rejected putting the trait in `exec`: then `agent` would depend on `exec`
  for a secret type, the wrong direction for an agent-owned boundary.)

- **Topology (Q2 / AQ-6 = a).** ONE `BinanceSpotExecClient`; testnet vs mainnet is a
  typed `Network` injected at construction, **never a compile-time split and never a
  hard-coded URL** (the `binance.rs:128-133` `production()` anti-pattern F1 must not
  repeat). `endpoint.rs` resolves `Network::Testnet → "https://testnet.binance.vision"`
  and `Network::Mainnet → "https://api.binance.com"`, each carrying a greppable
  `label` ("testnet"/"mainnet") for the arming audit F2 will want. **F1 ships
  testnet-only:** the default constructor yields `Network::Testnet`, and a unit test
  asserts the default endpoint label is `"testnet"` (so "F1 ships testnet-only" is
  *enforced*, not hoped). Mainnet is gated by the F2 arming guard, **not** by the
  client type — so every testnet rehearsal exercises the exact mainnet code path
  (no "tested testnet, shipped untested mainnet" gap).

### A2 — `SecretSource` trait shape + two impls + the fails-closed constructor contract (R2)

> **Binding law 1 (verbatim):** No secrets in git, ever — the safe path is the only path.

```rust
// crates/core::secret  (trait + newtype; no deps) — the shared secret vocabulary.
pub trait SecretSource: Send + Sync {
    /// Returns Err(SecretError::Missing) when absent — NEVER a default/empty key.
    fn get(&self, key: &str) -> Result<SecretString, SecretError>;
    /// Presence-only probe for the F2 arming guard condition (4); never reads the value.
    fn has(&self, key: &str) -> bool { self.get(key).is_ok() }
}
pub struct SecretString(/* private */ String);     // Debug/Display ⇒ "<redacted>"
pub enum SecretError { Missing(String), Io(String) }
```

- **`SecretString` redaction (AC-2).** `Debug` and `Display` both emit `"<redacted>"`;
  `Serialize` is **refused** (returns a ser error, never the plaintext) — there is no
  code path that prints, logs, or serializes the value. The plaintext is reachable
  **only** via an explicit `expose_secret(&self) -> &[u8]` consumed solely by
  `sign.rs` (borrowed, never stored, never copied into any struct's `Debug`).
- **Two impls in `crates/agent::secret`.** `EnvSecretSource` reads `BINANCE_API_KEY` /
  `BINANCE_API_SECRET` from the process env (never touches repo disk).
  `LocalFileSecretSource` reads the git-ignored `config/agent.toml.local` — the
  **proven** LLM-key precedent (`config.rs:612-651`), never a new mechanism; the
  committed config carries only placeholders.
- **Fails-closed constructor contract (AC-3).** `BinanceSpotExecClient::connect(network,
  secrets: &dyn SecretSource, http)` calls `secrets.get("BINANCE_API_KEY")` +
  `get("BINANCE_API_SECRET")` and **returns `Err` if either is `Missing`** — never a
  default key, never an empty key, never a silent unauthenticated request. There is
  **no API to pass a literal key** in code or committed config: `get` is the sole
  ingress. (This is exactly the presence the F2 arming condition (4) reads via
  `secrets.has(..)` — F1 builds it; F2 composes it.)

### A3 — Reconciliation loop: owner, the two-class divergence contract (AQ-1), and kill-switch wiring

> **AQ-1 is the riskiest decision in F1 — it is the safety-tolerance + halt-trigger
> calibration on real money. A false-halt mid-rebalance erodes operator trust; a
> missed real divergence is silent real-money drift. Both are load-bearing, so the
> contract encodes the two genuinely-different failure classes rather than one flat
> number.** ACCEPT recommended default (a), pinned precisely below.

**Owner: the reconciler in `crates/agent` (NOT `crates/exec`).** The reconciler
already owns the kill-switch handle and the per-bar `after_bar_close` cadence
(`reconciler.rs:71-89`); the exec crate stays a pure transport. F1 adds an
`Option<Arc<dyn AccountReader>>` field to `ReconcilerTask` (mirroring the existing
`Option<Arc<dyn LiveEquityStore>>` mode-gated field) + an `Option<DivergenceState>`
debounce counter. **Paper/research is byte-unchanged when `AccountReader = None`** —
the self-referential heuristic at `reconciler.rs:222-229` stays verbatim for paper
(A4 assumption; AC-10 second half). The real-exchange comparison runs **only** when
`Some`.

**What is compared, at what cadence, and what each divergence class does:**

| Item | Source A (in-process / ledger) | Source B (exchange truth) | Compared as |
|------|-------------------------------|---------------------------|-------------|
| Per-asset balance | ledger free+locked per `Asset` | `AccountReader::account_snapshot().balances[asset]` (free+locked) | `Decimal` delta, valued at last mark → USDT notional |
| Position presence | the set of assets the ledger knows | the set of non-dust assets the exchange reports | **set membership** |

- **Cadence.** The reconciliation compare runs on the reconciler's existing
  **per-bar `after_bar_close`** tick in live mode (the same cadence that already
  marks equity), **not** a separate timer — one account read per closed bar. (The
  passive baseline is low-turnover ~12–13 acts/year, so per-bar account reads on the
  monitoring cadence are well within rate limits; no extra timer to reason about.)

- **Class SOFT — transient balance timing (debounced, N=2).** A per-asset balance
  delta whose **absolute USDT-valued magnitude exceeds `[live].reconcile_tolerance_usdt`**
  (a `Decimal`, default `dec!(1.00)` — one dollar) but where **both sides know the
  asset** (the position is *not* unknown). This is the benign class: an in-flight fill
  the exchange has applied but the ledger has not yet recorded (or vice-versa). It is
  **debounced**: a `DivergenceState { consecutive: u8 }` counter increments on each
  divergent read and **resets to 0 on any in-tolerance read**. The
  `HaltReason::LedgerImbalance` trip fires only on the **N-th consecutive** divergent
  read (`[live].reconcile_debounce_reads`, default **2**). One transient mismatch that
  self-heals on the next bar does **not** halt — this is the false-halt guard.

- **Class HARD — structural unknown position (immediate, N=1, no debounce).** The
  exchange reports a **non-dust position in an asset the ledger has zero record of**
  (set-membership B ⊄ A) — OR the ledger believes it holds an asset the exchange
  reports at zero when the ledger qty is non-dust (A ⊄ B on a *position*, not a
  rounding-dust delta). **An unknown position is never benign**: it means the agent's
  model of what it holds is structurally wrong, which on real money is a
  stop-everything condition. This trips `HaltReason::LedgerImbalance`
  **immediately, bypassing the debounce counter** (the counter is for magnitude
  timing, not for "I don't know what I own"). The "dust" floor reuses
  `[live].reconcile_tolerance_usdt` so a 0.0000001-BTC rounding crumb is not mistaken
  for an unknown position.

  > **The trade-off, made explicit (AQ-1 risk note).** SOFT-debounced-N=2 costs us at
  > most one extra bar of latency before halting on a *sustained* balance drift (≈ one
  > minute on 1m bars) in exchange for not false-halting on the single-bar in-flight-fill
  > race that *will* occur during a live rebalance. HARD-immediate accepts zero latency
  > tolerance for the one class where latency tolerance would be reckless (an unknown
  > position). The flat-single-number fallback (AQ-1 option b) was rejected because it
  > forces one knob to serve both classes: tight enough to catch drift ⇒ false-halts on
  > the first in-flight fill; loose enough to ride the fill ⇒ misses a real drift. The
  > two-class split is the durable encoding (no v0.2.0 "debounce the reconciler" follow-on).

**Audit row each transition writes (the journal seam).** Every reconcile transition
writes through the existing `audit::journal::strategy_event` dual-write seam (the
same `strategy_events` + memo path the kill-switch trip already uses,
`kill_switch.rs:287-301`). Timestamps are **6-digit fractional-second** (ADR-0004,
never Rfc3339-second — avoids SQLite ORDER BY ties). **No balance/key value that
could be a secret is ever in a memo — only asset symbols, `Decimal` deltas, and the
class.** The transitions:

| Transition | Event | Memo (Decimal-valued; no secrets) |
|------------|-------|-----------------------------------|
| SOFT divergence observed (counter 1..N-1) | `ReconcileDivergenceObserved` | `asset=BTC delta_usdt=2.30 consecutive=1/2 class=soft` (no trip yet) |
| SOFT divergence trips (N-th consecutive) | `ReconcileDivergenceHalt` | `asset=BTC delta_usdt=2.30 consecutive=2/2 class=soft → LedgerImbalance` |
| HARD unknown position (immediate) | `ReconcileDivergenceHalt` | `asset=DOGE qty=120.0 class=hard_unknown_position → LedgerImbalance` |
| Counter reset (back in tolerance) | `ReconcileDivergenceCleared` | `asset=BTC delta_usdt=0.01 consecutive_reset` |

On either `…Halt` row the existing `KillSwitch::trip(HaltReason::LedgerImbalance)`
fires its full side-effect chain (broadcast `Halted` + the journal `kill_switch_tripped`
dual-write + incident-report spawn, `kill_switch.rs:274-313`) — F1 wires the
**trigger**, reusing the proven trip machinery verbatim.

### A4 — `ExecError` taxonomy + Binance-code mapping (R6) + the retry/idempotency contract

The existing `ExecError` (`router.rs:8-15`, three variants) is **extended additively**
(the paper `UnsupportedMode`/`OrderRejected`/`FillFailed` variants stay so
`PaperExecRouter` is untouched). New variants and their Binance-code mapping:

| `ExecError` variant | Triggered by | Retry policy |
|---------------------|--------------|--------------|
| `Transport(String)` | reqwest timeout / connect failure | **query status before any retry** (AC-8); never blind-resubmit |
| `RateLimited { retry_after }` | HTTP 429 / Binance `-1003` | capped exponential backoff, **hard ceiling**; then halt |
| `Auth(String)` | `-1022` (bad sig) / `-2014` / `-2015` (key) | **no retry** — fail fast (a key/sig fault won't self-heal) |
| `ClockSkew` | `-1021` (timestamp outside recvWindow) | resync `GET /api/v3/time`, retry **once**; persistent → `HaltReason::ClockSkew` |
| `FilterReject(String)` | client-side R4 checks **AND** exchange `-1013` / `-2010` | **no retry**; force-refresh the filter cache (AQ-2) on the exchange-side variant |
| `InsufficientBalance` | `-2010` insufficient balance | no retry; surface to caller |
| `UnsupportedOrderType` | `OrderKind::Limit` handed to the F1 MARKET-only client | no retry (typed reject, never silent — AQ-3) |
| `CapExceeded { notional, cap }` | `check_notional_cap` (R8) | no retry (rejected before the network — AC-11) |
| `Unknown(String)` | any unmapped code | no retry; log + surface |

- **Idempotency (R6).** Every `place_order` mints a `newClientOrderId` (a UUID — the
  `OrderId` already on `Order`, `order.rs:15`, reused) so a retry after an ambiguous
  outcome is **idempotent at the exchange**.
- **Ambiguous-timeout contract (AC-8 — adversarial).** On a `Transport` timeout from
  a `place_order`, the client **queries `GET /api/v3/order` by `newClientOrderId`
  BEFORE any retry**. If the order exists (filled/partial/new) it is **not**
  resubmitted (the ack is reconstructed from status); only a confirmed "does not
  exist" permits a resubmit. Partial fills are tracked on the returned status.
  **N-retry exhaustion → log + `halt`, never silent** (R6).
- **No `f64` anywhere in this path** (AC-9): backoff arithmetic uses integer
  millis/`Duration`; all notionals/caps/balances are `Decimal`.

### A5 — Exec-side cap mechanism (R8 / AQ-4 = a) — the F1 half of the F1/F2 seam

> **Binding law 4 (verbatim):** the operator arms; the agent never self-arms. **F1
> builds the cap mechanism; F1 decides nothing armed.**

`crates/exec/src/live/cap.rs` ships a **standalone pure fn**:

```rust
pub fn check_notional_cap(order_notional: Decimal, cap: Decimal) -> Result<(), ExecError> {
    if order_notional > cap { Err(ExecError::CapExceeded { notional: order_notional, cap }) }
    else { Ok(()) }   // notional == cap is ALLOWED (boundary)
}
```

plus the `[live].max_notional_usdt` config field parse (a `Decimal`). F1 builds and
**unit-tests the cap arithmetic + the rejection path** (AC-11, parametrized over
(notional, cap) incl. the `notional == cap` boundary allowed / `notional > cap`
rejected; the faked transport records **zero** requests for the rejected case). F1
**never** reads the arm-file, never checks mode, never composes the 5-condition
guard, never reads secret *presence* for arming — that is **F2-T3's `check_armed`**,
which *calls* `check_notional_cap` as its condition (3) (ADR-0054 § D2; umbrella A4).
The cap is proven in isolation in F1 so F2 composes a proven part rather than
re-deriving the arithmetic inside its adversarial matrix.

### A6 — Test strategy: what is offline-unit, what is trait-faked-transport, what is `#[ignore]`-live

> **Binding law 3 (verbatim):** every external I/O behind a trait; **no F1 test ever
> touches a real exchange or a real key** (AC-12). **Zero mainnet calls in CI, ever.**

**Dependency decision (checklist applied — see § Library compatibility below): the
PRIMARY test layer is a trait-faked transport, NOT a mock HTTP server.** A
`FakeTransport` test double implementing `LiveExecRouter` / `AccountReader` (and a
`FakeSecretSource`) beats adding/using `wiremock` for the adversarial matrix:
faking at the trait boundary is exactly invariant iii, needs **zero new deps**, and
asserts the load-bearing facts directly (e.g. "the faked transport recorded **zero**
outbound requests" for a filter/cap reject — AC-5/AC-11). `wiremock` **is already a
workspace dev-dep** (used by `data`/`llm`/`trader`) and **may** be used for a small
number of HTTP-shape tests (verifying the signer's query string + headers actually
hit the wire as expected) — but it is **not** required and the adversarial matrix
does not depend on it. **No new dependency is introduced for testing.**

| Layer | What it covers | Mechanism (no real exchange, no real key) |
|-------|----------------|-------------------------------------------|
| **Offline unit** | signer fixed-vector (AC-6); filter round/validate math (AC-5); cap arithmetic (AC-11); `Decimal`-only static grep (AC-9); error-code→variant mapping (R6); endpoint default = testnet (AQ-6); `SecretString` redaction (AC-2); fails-closed ctor (AC-3) | pure fns + recorded JSON fixtures; FAKE placeholder keys only |
| **Trait-faked transport** | order observably submitted once + `newClientOrderId` + sig (AC-7); ambiguous-timeout-queries-before-resubmit (AC-8); reconcile divergence → halt, both SOFT-debounce and HARD-immediate (AC-10); paper byte-unchanged when `AccountReader=None` | `FakeTransport`/`FakeSecretSource`/`FakeAccountReader` impls; records calls, dials nothing |
| **`#[ignore]` live testnet** | the full place→status→cancel→account-read→reconcile pipeline on fake money (AC-13) | a `#[ignore]`-gated integration suite the **operator** runs with their out-of-band testnet keys; **never in CI** |

- **Signer fixed-vector (AC-6).** `sign.rs` is a pure `(secret, query) -> hex`
  function unit-tested against a **pinned** vector using a **FAKE** secret — either
  the Binance API-docs public example pair (documented as an example, never a live
  credential) or a synthetic `secret="FAKE_TESTNET_SECRET_DO_NOT_USE"` with the
  expected signature computed once and pinned. **No file in `crates/` or `spec/`
  carries a real key.**
- **No-real-exchange/no-real-key CI gate (AC-12 — load-bearing).** Every CI test runs
  with `BINANCE_API_KEY`/`_SECRET` **unset** and passes; all transport is the faked
  trait or recorded JSON; the testnet host string appears **only as data** (never
  dialed). A test asserts the default endpoint is `testnet` and **no test references
  `api.binance.com`**. The `#[ignore]` live suite is the only thing that ever opens a
  socket, and it is operator-run, never CI.

**The `#[ignore]` live testnet integration suite — env contract + watch recipe it emits.**
The suite (e.g. `crates/exec/tests/binance_testnet_live.rs`, every test
`#[ignore]`-gated) reads its config **only** from the environment so no key is ever a
fixture or an arg:

| Env var | Meaning | Notes |
|---------|---------|-------|
| `BINANCE_TESTNET_API_KEY` | operator-provisioned **testnet** key | never logged; via `EnvSecretSource` |
| `BINANCE_TESTNET_API_SECRET` | operator-provisioned **testnet** secret | never logged; redacted |
| `BINANCE_EXEC_LIVE_TESTNET=1` | opt-in toggle | absent ⇒ the suite is a no-op even with `--ignored` |

The suite is `Network::Testnet`-pinned (it will **refuse to run against mainnet** —
asserts the endpoint label is `"testnet"` before the first request). When the
developer wires it they emit, per the watch-recipe contract, a copy-pasteable block
for the **operator** to run out-of-band (this is the AC-13 recipe's executable core;
the assistant produces it and **never runs it**):

```
# OPERATOR-ONLY — fake testnet money. The assistant never runs this.
export BINANCE_TESTNET_API_KEY=…        # your testnet key, provisioned out-of-band
export BINANCE_TESTNET_API_SECRET=…     # your testnet secret
export BINANCE_EXEC_LIVE_TESTNET=1
cargo test -p exec --test binance_testnet_live -- --ignored --nocapture
# Expected: place→status→cancel→account-read→reconcile all green on testnet.binance.vision;
#           the reconcile compare matches (no LedgerImbalance); no mainnet host ever dialed.
```

### Determinism & format guardrails this design binds (for the tester)

- **`Decimal` everywhere** (AC-9): no `f64` in any order/balance/cap/filter/tolerance/
  rounding type or arithmetic path. Backoff uses integer `Duration`.
- **Audit timestamps 6-digit fractional second** (ADR-0004) on every reconcile row —
  never `Rfc3339` second precision.
- **No RNG in the F1 decision path.** `newClientOrderId` is a `Uuid` (identity, not a
  sampled value); if any seeded randomness were ever needed it would be
  `ChaCha20Rng::from_seed`. The signer is a pure function.
- **Anchor-neutral by construction** (AC-15): the live client is **never** on the
  hashed backtest-report path (the backtest never calls it), so F1 mutates **no**
  `anchors.toml` row and **no** anchor SHA. Any anchor change would require its own
  ADR — none is needed.

### Library / crate compatibility checklist (new deps this design needs)

F1 needs HMAC-SHA256 signing in `crates/exec`, which currently has **no** `reqwest`,
`rust_decimal`, `hmac`, or `hex`. Decisions (each checklist item verified):

| Dep | Status in workspace | Checklist verdict |
|-----|---------------------|-------------------|
| `reqwest` 0.12 (`json`) | workspace dep (used by `data`/`agent`) | **REUSE** — add to `exec`'s `[dependencies]`. Single-binary-friendly (no separate service); no system C dep with `rustls` (the existing feature set); edition-2024 clean (already compiled in the tree). |
| `rust_decimal` | workspace dep (everywhere) | **REUSE** — promote from `exec` dev-dep to `[dependencies]` (the money-math rule). |
| `sha2` 0.10.8 | **already** a workspace dep | **REUSE** for HMAC's hash. |
| `hmac` | **NOT present** | **ADD** (RustCrypto, MIT/Apache-2.0, maintained, edition-2024 clean, pure-Rust no system C dep, `name="hmac"` shadows no stdlib crate). Pin `^0.12` (the `sha2 0.10` companion). |
| `hex` | **NOT present** | **ADD** (MIT/Apache-2.0, ubiquitous, pure-Rust, no system C dep, `name="hex"` no stdlib shadow) for the signature hex-encode. |
| `serde_json` 1.0 | **already** a workspace dep | **REUSE** for the REST JSON (ack/status/account/exchangeInfo). |
| `wiremock` 0.6.2 | **already** a workspace dev-dep | **OPTIONAL dev-dep** — not required (trait-faked transport is primary); may be used for a few HTTP-shape tests. **No new dep.** |

**Rejected:** adding a Binance SDK crate (e.g. `binance-rs`) — pulls a venue-coupled
dependency surface, most are stale (> 18 mo) and/or pin their own HTTP client; F1's
authenticated surface is **four** signed endpoints, hand-rolled over the existing
`reqwest`/`sha2` is smaller, auditable, and edition-2024-clean. The architect records
`hmac`/`hex` as the only two **new** crates; both are RustCrypto-ecosystem,
single-binary-friendly, no system C deps, license-compatible. **The developer adds
them to `crates/exec/Cargo.toml` only** (not the virtual-workspace root unless a
second crate needs them).

### Confirmation — baseline-equity-divergence gate is **N/A for F1** (re-ruled, NOT rubber-stamped)

Per the `v3-volatility-forecaster-noop-fix` precedent (rule the gate explicitly,
every time): **F1 ships no sizing decision.** It is a transport/client feature — it
places the order it is handed, reads balances, reconciles. There is no allocation, no
weight, no rebalance, no `scale` "computed but never applied" — the gate has **no
decision variable to bind** in F1. F1's analogue of "the order actually left the
process" is discharged by **AC-7** (a placed order is observably submitted through
the faked client) + **AC-11** (an over-cap/under-filter order observably does **not**
reach it) + the testnet rehearsal **AC-13**. **The gate APPLIES to F2** (ADR-0054
§ D4: the equal-weight inception allocation + monthly rebalance is the sizing
decision; e2e proof `passive_inception_diverges_from_flat_baseline`). This split is
the honest read — you cannot prove "the allocation moved capital" in a feature with
no allocation. **The tester records N/A-for-F1 with this justification, not a stamp.**

## Backtest Scenarios
_N/A — F1 is a transport/client feature with no strategy, no sizing, and no backtest
surface. The live client is never invoked on the backtest path (AC-15, anchor-neutral).
The F1 verification surface is unit tests + faked-I/O integration tests + the testnet
rehearsal recipe (AC-13), NOT a backtest scenario._

## Implementation
_developer fills this (M-DEV)._

## Verification
_tester links to reports here (M-TEST). Gate on the adversarial AC set by name:
AC-2 (secret redaction), AC-5 (filter fail-fast), AC-7 (order observably submitted),
AC-8 (timeout queries before resubmit), AC-9 (Decimal-only), AC-10 (reconcile→halt),
AC-11 (exec-side cap), AC-12 (no real exchange/key in CI). Confirm AC-14 (parse-
rejection untouched) + AC-15 (anchor-neutral). The tester records the baseline-
divergence gate as N/A-for-F1 with the § non-negotiable-4 justification (NOT a
rubber-stamp)._

## Changelog

- 2026-06-12 (analyst): created F1 feature brief (v0.1.0, draft) under the
  `live-passive-execution-readiness` umbrella, dispatched after P0 (ADR-0054 § D7)
  ratified 2026-06-12. Scoped F1 as the testnet-first execution **substrate** only —
  `LiveExecRouter` + `BinanceSpotExecClient` (MARKET/status/cancel, ONE client URL+
  keys injected per Q2), `SecretSource` (env + git-ignored local file per Q6; safe
  path the only path; no key material in any fixture), `AccountReader`, exchange-
  filter pre-validation (`Decimal`, fail-fast), the real-exchange reconciliation loop
  (replaces `reconciler.rs:222-229` self-ref; divergence → `LedgerImbalance` halt),
  HMAC signing + clock-skew, and the error/retry/idempotency taxonomy (query-before-
  retry on ambiguous timeout). Built 8 R + 15 AC (8 adversarial: secret-redaction,
  filter-fail-fast, order-observably-submitted, timeout-query-before-resubmit,
  Decimal-only, reconcile→halt, exec-side-cap, no-real-exchange/key-in-CI) + 6 architect
  questions (AQ-1 reconcile-tolerance+debounce flagged riskiest). Stated the binding
  CLAUDE.md non-negotiables (external-I/O-behind-trait, no-secrets-in-git, Decimal-
  never-f64) and JUSTIFIED the baseline-equity-divergence gate as **N/A for F1** (no
  sizing decision — transport feature; the gate APPLIES to F2 per ADR-0054 § D4) — not
  rubber-stamped. Named the F1/F2 exec-side-cap seam (F1 builds + unit-tests the cap
  arithmetic; F2's `check_armed` composes it). Created the `REQ-LIVE-EXEC-CLIENT-001`
  feature-level trace row (appendix in tasks.md for the orchestrator to apply; cross-
  references the umbrella `REQ-LIVE-PASSIVE-EXEC-F1-001`). No code, no keys, no config,
  no git.
- 2026-06-12 (architect): M-T1 design pass → `## Architecture` (A1–A6), status
  `draft → arch-done`, version `0.1.0 → 0.2.0`. Resolved all 6 AQs by ACCEPTing each
  recommended default with code-grounded reasons: **AQ-1** (riskiest) two-class
  divergence — per-asset `Decimal` tolerance `[live].reconcile_tolerance_usdt`
  (default `dec!(1.00)`) on USDT-valued free+locked balance deltas, SOFT class
  debounced N=2 consecutive reads (`[live].reconcile_debounce_reads`), HARD class
  (unknown exchange position / structural set-membership mismatch) immediate N=1
  no-debounce → `HaltReason::LedgerImbalance`; pinned exactly what is compared
  (per-asset free+locked + position presence), cadence (per-bar `after_bar_close` in
  live), per-class action, and the 4 audit-row transitions
  (`ReconcileDivergenceObserved`/`…Halt`/`…Cleared` via `strategy_event`, 6-digit
  fractional ts, no secrets in memos); the false-halt-vs-missed-drift trade-off made
  explicit. **AQ-2** TTL-cached filters (1 h) + force-refresh on exchange `-1013`/`-2010`.
  **AQ-3** MARKET-only — `Order` already carries `OrderKind::{Market,Limit}` (core
  `order.rs:39`) so LIMIT is additive natively; F1 rejects `OrderKind::Limit` with a
  typed `ExecError::UnsupportedOrderType`. **AQ-4** standalone `check_notional_cap` +
  `[live].max_notional_usdt` parse, unit-tested in F1; F2's `check_armed` composes it.
  **AQ-5** `SecretSource`/`SecretString` **impls** in `crates/agent::secret`; the
  **trait** declared in `crates/core::secret` (no-dep shared vocabulary) so
  `crates/exec` consumes `&dyn SecretSource` with **no `exec→agent` cycle** — the
  dependency-direction-clean realization of (a). **AQ-6** typed `Network::{Testnet,
  Mainnet}` → base_url + greppable label, default Testnet, a unit test asserts the
  default endpoint is testnet (F1-testnet-only enforced, not hoped). **ADR decision:
  NO new ADR** — ADR-0054 § D1/D2/D3 already fixes the boundary/arming-contract/
  invariants; F1 is implementation under it; the AQ resolutions are choices within
  the ratified boundary, not boundary moves; the only ADR-requiring act (anchor-SHA
  mutation) is explicitly not crossed (AC-15). Locked the `crates/exec/src/live/`
  module layout (client/sign/clock/filters/cap/error/endpoint/types), the additive
  `ExecError` taxonomy + Binance-code mapping, the `Option<Arc<dyn AccountReader>>`
  reconciler seam (paper byte-unchanged when `None`), and the test strategy (primary:
  trait-faked transport — ZERO new deps; `wiremock` already a workspace dev-dep is
  optional; `#[ignore]`-gated operator-run testnet suite with its env contract +
  watch recipe). Dep checklist: REUSE `reqwest`/`rust_decimal`/`sha2`/`serde_json`;
  ADD only `hmac`+`hex` (RustCrypto, MIT/Apache, edition-2024 clean, no system C dep,
  no stdlib shadow) to `crates/exec` only; REJECT a Binance SDK crate. Re-ruled the
  baseline-equity-divergence gate **N/A for F1** (transport, no sizing decision —
  AC-7/AC-11/AC-13 discharge the "order left the process" analogue; gate APPLIES to
  F2 per ADR-0054 § D4) — NOT rubber-stamped. Reproduced the five binding invariants
  verbatim in the `## Architecture` § Binding law. **No disagreement with the
  analyst** — every recommended default accepted; the deltas are sharpenings (the
  AQ-5 trait-in-core dependency-cycle fix; the AQ-1 audit-row + cadence pin; the
  enforced testnet-default test). No code, no keys, no config, no git.
- 2026-06-12 (presenter): assembled the release deck
  `presentations/live-exec-client-binance-spot-2026-06-12.md` (operator approval gate)
  after tester `VERDICT → PASS`. Advanced status `tester-done → presenter-done` (the
  presenter-owned cycle-completion frontmatter advance the spec-lint status-drift rule
  requires once a deck exists alongside a PASS report). Evidence re-verified live at
  HEAD `414c18a`: anchors 119/119, exec adversarial 7/7, reconciler two-class 7/7,
  `t12_mode_live_is_rejected` PASS, testnet suite clean-skip (0/3); security greps
  (mainnet URL confined to the `Network::Mainnet` enum arm, zero signature logging,
  testnet default) re-run clean. No code, no keys, no config, no git.

## Implementation

**Implemented by developer, 2026-06-12. All waves A–F1 complete.**

### Files created / modified

| File | Role |
|------|------|
| `crates/core/src/secret.rs` | `SecretSource` trait, `SecretString` newtype, `SecretError` enum (Wave A1) |
| `crates/core/src/lib.rs` | Added `pub mod secret;` + re-exports |
| `crates/core/src/asset.rs` | Added `PartialOrd, Ord` to `Asset` derive (required for `BTreeMap<Asset, Balance>`) |
| `crates/agent/src/secret.rs` | `EnvSecretSource`, `LocalFileSecretSource` impls (Wave A2) |
| `crates/agent/src/lib.rs` | Added `pub mod secret;` |
| `crates/exec/src/live/endpoint.rs` | `Network::{Testnet, Mainnet}` enum, `Default → Testnet` (Wave B1) |
| `crates/exec/src/live/sign.rs` | Pure `sign(secret, query) -> String` HMAC-SHA256+hex (Wave B1) |
| `crates/exec/src/live/error.rs` | Full `ExecError` taxonomy + `map_binance_code` (Wave B2) |
| `crates/exec/src/live/clock.rs` | `ServerTimeOffset`, `MAX_SKEW_MS`, `adjusted_now_ms`, `sync`, `check_persistent` (Wave B4) |
| `crates/exec/src/live/types.rs` | `OrderRef`, `OrderAck`, `OrderStatus`, `Balance`, `AccountSnapshot` + Binance serde shapes (Wave B3/D1) |
| `crates/exec/src/live/filters.rs` | `ExchangeFilters`, `FilterCache` (1h TTL), `round_to_step`, `validate_order`, `parse_filters_from_json` (Wave C1) |
| `crates/exec/src/live/cap.rs` | `check_notional_cap` pure fn (Wave C2) |
| `crates/exec/src/live/mod.rs` | `LiveExecRouter` + `AccountReader` traits, `BinanceSpotExecClient` (Waves B3, D1, D2) |
| `crates/exec/src/lib.rs` | Added `pub mod live;` + re-exports |
| `crates/exec/src/router.rs` | Moved `ExecError` to `live/error.rs` (additive, paper variants untouched) |
| `crates/exec/Cargo.toml` | Added `hmac 0.12`, `hex 0.4`, `reqwest`, `rust_decimal`, `rust_decimal_macros`, `uuid`, `time`, `serde_json` |
| `Cargo.toml` | Added `hmac` + `hex` to workspace deps |
| `crates/agent/src/reconciler.rs` | Two-class SOFT/HARD reconciliation, `KillSwitch::trip(LedgerImbalance)` (Wave E1) |
| `crates/exec/tests/live_exec_adversarial.rs` | 7 adversarial CI tests via `FakeTransport` + `FakeAccountReader` |
| `crates/agent/tests/live_reconcile_adversarial.rs` | 7 reconciler adversarial tests |
| `crates/exec/tests/binance_testnet_live.rs` | 3 `#[ignore]`-gated live testnet suite tests (Wave F1) |

### Security invariants met

1. **No secrets in git, no key material in any fixture** — all test keys are `"FAKE_TESTNET_KEY_DO_NOT_USE"` or Binance public docs example keys. Grep-clean verified.
2. **Zero mainnet calls in CI/tests** — `Network::default() == Testnet` enforced by `default_endpoint_is_testnet` test; FakeTransport is primary test layer; testnet suite is `#[ignore]`-gated + `BINANCE_EXEC_LIVE_TESTNET=1` toggle.
3. **Fails closed** — `BinanceSpotExecClient::connect` returns `Err(ExecError::Auth)` when either key is `Missing`; `no_real_exchange_no_real_key_in_ci` test verifies this.
4. **`mode=live` parse-rejection unchanged** — `config.rs:660-668` + `t12_mode_live_is_rejected` untouched.
5. **`Decimal` everywhere** — no `f64` in any money/price/qty/filter/tolerance path; `#![deny(clippy::float_arithmetic)]` in exec lib enforces.
6. **`SecretString` never logged** — `Debug`/`Display` emit `<redacted>`; `Serialize` refused.

### Test inventory (38 exec + 7 agent reconciler + 3 ignored testnet = 48 new tests)

Adversarial AC coverage:

| AC | Test(s) | File |
|----|---------|------|
| AC-2 (secret-redacted) | `secret_never_logged_or_serialized`, `has_proxies_get` | `crates/core/src/secret.rs` |
| AC-3 (fail-closed) | `missing_secret_fails_closed_env`, `empty_env_var_is_missing`, `missing_secret_fails_closed_local_file`, `local_file_reads_value` | `crates/agent/src/secret.rs` |
| AC-5 (filter-fast-fail) | `under_min_notional_fails_fast`, `bad_lot_step_rejected` | `crates/exec/src/live/filters.rs` |
| AC-6 (sig-vector) | `signer_reproduces_fixed_vector`, `signer_fake_key_vector` | `crates/exec/src/live/sign.rs` |
| AC-6/R5 (clock-skew) | `clock_skew_resyncs_then_halts`, `adjusted_now_ms_is_reasonable`, `sync_updates_offset` | `crates/exec/src/live/clock.rs` |
| AC-7 (order-once) | `order_observably_submitted_once` | `crates/exec/tests/live_exec_adversarial.rs` |
| AC-8 (timeout-query) | `ambiguous_timeout_queries_before_resubmit` | `crates/exec/tests/live_exec_adversarial.rs` |
| AC-10a (diverge-halt) | `reconcile_divergence_trips_halt`, `soft_once_then_clear_no_halt`, `soft_divergence_counter_resets_on_clear`, `tolerance_boundary_exact_no_halt` | `crates/agent/tests/live_reconcile_adversarial.rs` |
| AC-10b (hard-trip) | `reconcile_unknown_position_hard_trips`, `hard_immediate_trips_on_first_read` | `crates/agent/tests/live_reconcile_adversarial.rs` |
| AC-11 (cap) | `exec_side_cap_rejects_over_notional`, `cap_exceeded_error_carries_values`, `cap_exceeded_fake_transport_receives_zero_requests` | `crates/exec/src/live/cap.rs` + adversarial test |
| AC-12 (no-mainnet) | `no_real_exchange_no_real_key_in_ci`, `default_endpoint_is_testnet` | `crates/exec/tests/live_exec_adversarial.rs` + `endpoint.rs` |
| AC-13 (testnet-rehearsal) | `place_order_testnet`, `account_read_testnet`, `reconcile_no_divergence_testnet` (all `#[ignore]`) | `crates/exec/tests/binance_testnet_live.rs` |

### Gate results (pre-handoff)

- `cargo test -p exec -p agent -p audit` — all pass (0 FAILED)
- `cargo test -p ui --lib` — 447 pass (untouched)
- `cargo clippy -p exec -p agent` — 0 errors, warnings are `#[warn(clippy::pedantic)]` only
- `cargo fmt --check -p exec -p agent -p trading_core` — clean
- `cargo deny check` — no warnings/errors for `hmac` or `hex`; pre-existing violations for `polars-error`/`paste` are not F1
- `bash scripts/verify_anchors.sh` — 119/119 PASS
- `python3 scripts/spec_lint.py` — 70 violations (all pre-existing, zero new)
