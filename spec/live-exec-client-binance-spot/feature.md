---
slug: live-exec-client-binance-spot
status: draft
owner: analyst
updated: 2026-06-12
version: 0.1.0
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

## Design
_architect fills this (M-T1). Resolve AQ-1..AQ-6; confirm the N/A-for-F1 /
APPLIES-for-F2 baseline-divergence ruling; lock the crate boundaries
(`crates/exec` for the client + cap mechanism, `SecretSource` placement per AQ-5),
the `ExecError` taxonomy shape, and the reconciler `Option<Arc<dyn AccountReader>>`
seam. Cite ADR-0054 + the umbrella A1/A2. The umbrella A1/A2/A6 already carry most of
this design at program granularity — the M-T1 pass tightens it to F1's module/file
boundaries + the six AQ resolutions._

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
