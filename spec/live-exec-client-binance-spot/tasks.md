---
slug: live-exec-client-binance-spot
status: arch-done
owner: architect
updated: 2026-06-12
version: 0.2.0
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
     │  [GATE TO F2: testnet rehearsal passes — M-DEV-F2 / AC-13 recipe green]
     ▼
F2  PassiveBaseline policy + 5-condition arming guard  (composes F1's cap mechanism)
```

- **Every safety-critical task names its adversarial AC** — the tester gates on those.
- **F1 builds the exec-side-cap mechanism (M-DEV-C2) but decides nothing "armed"** —
  the 5-condition guard is F2-T3 (AQ-4 fixes the seam).
- **Two tasks REQUIRE operator-provisioned testnet keys** (M-DEV-F1 wires the suite;
  M-DEV-F2 is the operator's out-of-band rehearsal run — the GATE TO F2). Every other
  task lands + passes with keys **unset** (AC-12).

## Architect M-DEV decomposition (2026-06-12) — waves, gates, and the AC map

> **This replaces the analyst seed F1-T1..F1-T8 with an ordered, dependency-aware
> wave plan.** The seed task IDs are preserved (M-DEV-N tasks cite their seed `F1-T*`)
> so the trace stays legible. Design is fully specified in
> [feature.md § Architecture A1–A6](feature.md) — every task points at its A-section.

### Developer-track decision — SINGLE track (honest read)

**One developer track, sequential waves.** F1 is effort **L** but it is a **tightly
coupled vertical**: the signer (`sign.rs`) is consumed by the client; the client is
consumed by `AccountReader`; `AccountReader` is consumed by the reconciler; the error
taxonomy threads through all of them. A split (e.g. "secret crate" ‖ "exec client")
would create a merge seam on `ExecError` and the `SecretSource` trait location — the
two things most likely to churn — for little parallelism gain on an L-sized feature
with one risky decision (AQ-1). The **only** genuinely-independent unit is the
`crates/core::secret` trait declaration (Wave A) + the cap pure-fn (`cap.rs`, Wave C),
both small; they are sequenced early so nothing blocks on them rather than split to a
second agent. **No ui-designer track** — F1 has no UI surface (the live cockpit is
F3's monitoring concern). If the orchestrator wants parallelism, the safe cut is
**Wave A `secret` foundation** as a tiny standalone PR, then a single track for B–F.

### Wave A — foundations (no network, no exchange) — `crates/core` + `crates/agent`

- [ ] **M-DEV-A1 — `SecretSource` trait + `SecretString` in `crates/core::secret`
  (seed F1-T2, part 1).** Declare `trait SecretSource { get; has }`, the
  `SecretString` newtype (private field), `SecretError::{Missing,Io}` in a new
  `crates/core/src/secret.rs` (no deps — the shared vocabulary so `exec` consumes it
  with **no `exec→agent` cycle**, per [feature.md § A1/A2, AQ-5](feature.md)).
  `SecretString` `Debug`/`Display` ⇒ `"<redacted>"`; `Serialize` refused; plaintext
  reachable only via `expose_secret(&self) -> &[u8]`.
  _Adversarial (AC-2): `secret_never_logged_or_serialized` — `Debug`/`Display` emit
  `<redacted>`; `tracing` + `serde` round-trips never leak the value; fixtures use
  obviously-fake placeholders only (`"FAKE_TESTNET_KEY_DO_NOT_USE"`)._
  _Gate (invariant i): no key material in any fixture._

- [ ] **M-DEV-A2 — `EnvSecretSource` + `LocalFileSecretSource` in `crates/agent::secret`
  (seed F1-T2, part 2).** `EnvSecretSource` reads `BINANCE_API_KEY`/`BINANCE_API_SECRET`
  from process env (never repo disk); `LocalFileSecretSource` reads the git-ignored
  `config/agent.toml.local` (the proven LLM-key precedent `config.rs:612-651`). The
  safe path is the only ingress — **no code API takes a literal key**.
  _Adversarial (AC-3): `missing_secret_fails_closed` — absent secret ⇒
  `Err(SecretError::Missing)`; never a default/empty key, never a silent
  unauthenticated request._ → **the `no-secret-fails-closed` AC.**
  _Gate: `[gitignore]` carries `config/agent.toml.local` (already true); no `.local`
  committed._

### Wave B — the authenticated transport core (`crates/exec/src/live/`) — offline-testable

- [ ] **M-DEV-B1 — `Network` endpoint + the HMAC signer (seed F1-T1, part 1).**
  `endpoint.rs`: `Network::{Testnet,Mainnet}` → `base_url` + greppable `label`
  ([feature.md § A1, AQ-6](feature.md)); **default Testnet**. `sign.rs`: a **pure fn**
  `sign(secret: &[u8], query: &str) -> String` (HMAC-SHA256 → hex) — borrows the
  secret, never stores it, never in any `Debug`. Add `hmac` + `hex` to
  `crates/exec/Cargo.toml` (REUSE `sha2`); promote `rust_decimal` + add `reqwest` to
  `[dependencies]` (see [feature.md § dep checklist](feature.md)).
  _Adversarial (AC-6): `signer_reproduces_fixed_vector` — pure fn reproduces a pinned
  `(query, secret)→signature` with a FAKE secret; the key never appears in any client
  `Debug`._ → **the `signature-vector-match` AC.**
  _Acceptance (AQ-6): `default_endpoint_is_testnet` — the default `Network` label is
  `"testnet"`; no test references `api.binance.com`._ → **part of `zero-mainnet-in-CI`.**

- [ ] **M-DEV-B2 — `ExecError` taxonomy + Binance-code mapping (seed F1-T6, part 1).**
  Extend the existing `ExecError` (`router.rs:8-15`) **additively** with the variants +
  mapping table in [feature.md § A4](feature.md) (`Transport`/`RateLimited`/`Auth`/
  `ClockSkew`/`FilterReject`/`InsufficientBalance`/`UnsupportedOrderType`/
  `CapExceeded`/`Unknown`). Paper variants untouched (`PaperExecRouter` compiles
  unchanged).
  _Acceptance: unit test `binance_code_maps_to_variant` (a table of `-1003`/`-1021`/
  `-1022`/`-2010`/`-2014`/`-2015`/`-1013` → expected variant)._ → **the
  `retry-taxonomy` AC (mapping half).**

- [ ] **M-DEV-B3 — `BinanceSpotExecClient` + `LiveExecRouter` impl (place/status/
  cancel, MARKET-only) (seed F1-T1, part 2).** The client struct
  `{ endpoint, http, signer, secrets, … }` over signed REST: `POST /api/v3/order`
  (MARKET), `GET /api/v3/order` (status), `DELETE /api/v3/order` (cancel). **ONE
  client, `Network`+keys injected** — never the `binance.rs:128-133` hard-coded-mainnet
  anti-pattern. `OrderKind::Limit` ⇒ `ExecError::UnsupportedOrderType` (typed reject,
  never silent — [AQ-3](feature.md)). The **fails-closed constructor**
  `connect(network, &dyn SecretSource, http)` returns `Err` if either key is `Missing`
  ([feature.md § A2](feature.md)).
  _Acceptance (AC-1): `LiveExecRouter` exists (place MARKET/status/cancel); the client
  impls it; a `FakeTransport` also impls it; base-URL+keys constructor-injected; no
  hard-coded venue URL._
  _Gate (AC-12): the suite uses `FakeTransport`/recorded JSON — NO real-exchange call,
  keys unset._

- [ ] **M-DEV-B4 — clock-skew handling (seed F1-T1, part 3).** `clock.rs`:
  `ServerTimeOffset` syncs to `GET /api/v3/time` on construction + on any `-1021`;
  persistent skew beyond threshold maps to `HaltReason::ClockSkew` (variant exists,
  `kill_switch.rs:53`).
  _Adversarial: `clock_skew_resyncs_then_halts` — a faked `-1021` triggers one resync
  + retry; a persistent `-1021` surfaces `ExecError::ClockSkew` → halt path._ → **the
  `clock-skew-handling` AC.**

### Wave C — pre-trade validation gates (offline pure-fn) — depend on B2 (`ExecError`)

- [ ] **M-DEV-C1 — exchange-filter ingestion + client-side pre-validation (seed F1-T4).**
  `filters.rs`: ingest `LOT_SIZE`(`stepSize`,`minQty`)/`MIN_NOTIONAL`/`PRICE_FILTER`
  (`tickSize`) from `GET /api/v3/exchangeInfo` into `ExchangeFilters` (all `Decimal`);
  `round_to_step()` + `validate()` in `Decimal` **BEFORE submit**; TTL cache (1 h,
  default) + force-refresh on exchange `-1013`/`-2010` ([AQ-2](feature.md)). Reuse the
  `binance.rs:226-242` parse shape.
  _Adversarial (AC-5): `under_min_notional_fails_fast` + `bad_lot_step_rejected` — a
  below-`minNotional`/below-`minQty`/off-`stepSize` order returns a typed
  `ExecError::FilterReject` and the `FakeTransport` records **ZERO** outbound
  requests._ → **the `filter-rejections` AC.**
  _Gate (invariant ii): all rounding `Decimal`, never `f64`._

- [ ] **M-DEV-C2 — exec-side cap mechanism (seed F1-T8 — the F1 half of the AQ-4
  seam).** `cap.rs`: standalone pure fn `check_notional_cap(order_notional: Decimal,
  cap: Decimal) -> Result<(), ExecError>` (`notional == cap` allowed; `notional > cap`
  ⇒ `ExecError::CapExceeded`) + the `[live].max_notional_usdt` config-field parse
  (`Decimal`). F1 builds + unit-tests **only** the arithmetic + rejection;
  **F2-T3's `check_armed` composes it** — F1 reads no arm-file/mode/secret-presence
  ([feature.md § A5](feature.md)).
  _Adversarial (AC-11): `exec_side_cap_rejects_over_notional` — parametrized over
  (notional, cap) incl. boundary `notional == cap` ALLOWED, `notional > cap` REJECTED;
  the `FakeTransport` records zero requests for the rejected case._ → **the
  `exec-side-cap` AC.**

### Wave D — `AccountReader` + retry/idempotency — depend on B3

- [ ] **M-DEV-D1 — `AccountReader` trait + impl (`GET /api/v3/account`, signed) (seed
  F1-T3).** `account_snapshot()` returns `balances: BTreeMap<Asset, Balance{free,locked:
  Decimal}>` ([feature.md § A1](feature.md)); impl on `BinanceSpotExecClient` (same
  signer + `SecretSource`).
  _Acceptance (AC-4): behind a trait; faked / recorded-JSON `GET /api/v3/account`
  fixture (synthetic balances, no identifiers, no keys); balances parsed `Decimal`,
  free+locked split preserved._

- [ ] **M-DEV-D2 — retry / idempotency / ambiguous-timeout (seed F1-T6, part 2).**
  `newClientOrderId` (reuse the `Order.id` UUID) for idempotency; 429/`-1003` capped
  exponential backoff with a **hard ceiling**; **on an ambiguous timeout, query
  `GET /api/v3/order` by `newClientOrderId` BEFORE any retry** — never blind-resubmit a
  possibly-filled order; partial-fill accounting; N-retry exhaustion → log + `halt`,
  never silent ([feature.md § A4](feature.md)). Backoff uses integer `Duration` (no
  `f64`).
  _Adversarial (AC-7): `order_observably_submitted_once` — a valid in-filter order is
  submitted exactly once to the `FakeTransport` with a `newClientOrderId` + valid sig;
  the `OrderAck` round-trips (F1's "did it actually leave" analogue)._
  _Adversarial (AC-8): `ambiguous_timeout_queries_before_resubmit` — a timed-out place
  is status-checked, NEVER blind-resubmitted; N-retry exhaustion → halt, never silent;
  429/`-1003` backs off with a capped ceiling._ → **completes the `retry-taxonomy` AC.**

### Wave E — the real-exchange reconciliation loop (`crates/agent`) — depends on D1

- [ ] **M-DEV-E1 — two-class reconciliation + kill-switch wiring (seed F1-T5 — the
  AQ-1 task, the riskiest in F1).** Add `Option<Arc<dyn AccountReader>>` +
  `Option<DivergenceState{consecutive:u8}>` to `ReconcilerTask`. On the per-bar
  `after_bar_close` tick **in live mode only**, compare per-asset free+locked balances
  (USDT-valued `Decimal` delta) + position presence (set membership) vs
  `AccountReader` truth ([feature.md § A3](feature.md)):
  **SOFT** (magnitude > `[live].reconcile_tolerance_usdt`, both sides know the asset) →
  debounce N=`[live].reconcile_debounce_reads` (default 2) consecutive reads, reset on
  any in-tolerance read, trip on the N-th; **HARD** (unknown exchange position /
  structural set-membership mismatch above the dust floor) → **immediate trip, bypass
  the counter**. Both trip `KillSwitch::trip(HaltReason::LedgerImbalance)`. Write the
  4 transition audit rows (`ReconcileDivergenceObserved`/`…Halt`/`…Cleared` via
  `audit::journal::strategy_event`, 6-digit fractional ts, **no secrets/balances that
  could be secret in memos** — only asset symbols + `Decimal` deltas + the class).
  _Adversarial (AC-10): `reconcile_divergence_trips_halt` — (a) a SOFT divergent
  `AccountReader` for N consecutive reads ⇒ trip + the `…Halt` audit row lands; a
  single transient read does NOT trip (debounce proven); (b) a HARD unknown-position
  `AccountReader` ⇒ **immediate** trip (no debounce)._ → **the `divergence-matrix` AC
  + the `unknown-position-hard-trip` AC (both halves named to this task).**
  _Gate: with `AccountReader = None` (paper) the `reconciler.rs:222-229` heuristic is
  **byte-unchanged** — assert via the existing `t26_*` tests staying green._

### Wave F — operator-gated testnet rehearsal (REQUIRES operator-provisioned testnet keys)

- [ ] **M-DEV-F1 — `#[ignore]`-gated live testnet integration suite (seed F1-T7,
  part 1).** `crates/exec/tests/binance_testnet_live.rs`, every test `#[ignore]`-gated,
  `Network::Testnet`-pinned (refuses mainnet — asserts the endpoint label is
  `"testnet"` before the first request). Reads config **only** from env
  (`BINANCE_TESTNET_API_KEY`/`_SECRET`, `BINANCE_EXEC_LIVE_TESTNET=1`) — no key is ever
  a fixture or an arg ([feature.md § A6](feature.md)). Exercises
  place→status→cancel→account-read→reconcile. **The assistant writes this suite; the
  assistant NEVER runs it.**
  _Gate (AC-12): this suite is the ONLY thing that ever opens a socket; it is
  **never in CI** (`--ignored` + the env toggle gate it)._
  > **⚠ REQUIRES OPERATOR ACTION — testnet keys.** This task's *code* lands without
  > keys, but its *green checkmark* gates on the operator running it out-of-band with
  > their own testnet keys (see M-DEV-F2). Mark M-DEV-F1 done when the suite is wired
  > + compiles; the **rehearsal pass** is M-DEV-F2.

- [ ] **M-DEV-F2 — testnet rehearsal recipe + operator run (seed F1-T7, part 2) —
  GATE TO F2.** Produce the self-contained human-verification recipe (Command / Steps /
  Timing / Expected-result / Failure-diagnosis / Cleanup) per the project recipe
  contract, whose executable core is the M-DEV-F1 watch block in
  [feature.md § A6](feature.md). The assistant **produces** the recipe; the **operator
  runs** it against `testnet.binance.vision` with FAKE operator-provisioned testnet
  keys.
  _Acceptance (AC-13): the recipe is self-contained + the operator confirms the full
  pipeline green on fake money. **GATE TO F2** — F2 does not start until this is green._
  > **⚠ REQUIRES OPERATOR ACTION — testnet keys + an out-of-band run.** This is a
  > human-verification gate, not a CI gate. Per the human-verification recipe contract,
  > the request to the operator MUST be the self-contained recipe (Command / Steps /
  > Timing / Expected / Failure-diagnosis / Cleanup). The assistant never executes it.

### The 8 adversarial ACs → task map (the tester gates on these by name)

| Adversarial AC | Named test | Task |
|----------------|-----------|------|
| `no-secret-fails-closed` (AC-3) | `missing_secret_fails_closed` | **M-DEV-A2** |
| `zero-mainnet-in-CI` (AC-12) | `default_endpoint_is_testnet` + suite-wide unset-keys/no-`api.binance.com` | **M-DEV-B1** (+ every wave) |
| `signature-vector-match` (AC-6) | `signer_reproduces_fixed_vector` | **M-DEV-B1** |
| `filter-rejections` (AC-5) | `under_min_notional_fails_fast` + `bad_lot_step_rejected` | **M-DEV-C1** |
| `exec-side-cap` (AC-11) | `exec_side_cap_rejects_over_notional` | **M-DEV-C2** |
| `divergence-matrix` (AC-10a) | `reconcile_divergence_trips_halt` (SOFT + debounce) | **M-DEV-E1** |
| `unknown-position-hard-trip` (AC-10b) | `reconcile_unknown_position_hard_trips` | **M-DEV-E1** |
| `clock-skew-handling` (AC-6/R5) | `clock_skew_resyncs_then_halts` | **M-DEV-B4** |
| `retry-taxonomy` (AC-7/AC-8/R6) | `order_observably_submitted_once` + `ambiguous_timeout_queries_before_resubmit` + `binance_code_maps_to_variant` | **M-DEV-D2** (+ B2) |

> AC-7 (`order-observably-submitted`) is F1's "did the order actually leave the
> process" analogue (the baseline-divergence gate is N/A for F1 — transport, no
> sizing — see the cross-cutting gates below).

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
- 2026-06-12 (architect): M-T1 decomposition (v0.1.0 → v0.2.0, status `draft →
  arch-done`). Replaced the analyst seed F1-T1..F1-T8 with **6 ordered waves**
  (A foundations → B transport core → C pre-trade gates → D account+retry →
  E reconciliation → F operator testnet rehearsal), each task citing its seed `F1-T*`
  + its [feature.md § A-section](feature.md) + a per-task gate. **Single developer
  track** (honest read — F1 is a tightly-coupled vertical; the only safe parallel cut
  is the tiny Wave A `core::secret` foundation; no ui-designer track — no UI surface).
  Mapped the **8 adversarial ACs** each to a named task/test: `no-secret-fails-closed`
  (M-DEV-A2), `zero-mainnet-in-CI` (M-DEV-B1 + every wave), `signature-vector-match`
  (M-DEV-B1), `filter-rejections` (M-DEV-C1), `exec-side-cap` (M-DEV-C2),
  `divergence-matrix` + `unknown-position-hard-trip` (both M-DEV-E1, the AQ-1 task),
  `clock-skew-handling` (M-DEV-B4), `retry-taxonomy` (M-DEV-D2 + B2). **Marked the two
  tasks that REQUIRE operator-provisioned testnet keys** (M-DEV-F1 wires the
  `#[ignore]` suite; M-DEV-F2 is the operator's out-of-band rehearsal run = the GATE
  TO F2) with the ⚠ human-verification-recipe contract; every other task lands +
  passes with keys unset. No code, no keys, no config, no git.

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
