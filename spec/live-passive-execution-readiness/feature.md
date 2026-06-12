---
slug: live-passive-execution-readiness
status: draft
owner: analyst
updated: 2026-06-12
version: 0.1.0
trace: REQ-LIVE-PASSIVE-EXEC-F1-001, REQ-LIVE-PASSIVE-EXEC-F2-001, REQ-LIVE-PASSIVE-EXEC-F3-001
---

# Live passive-execution readiness — road-scoping for live-money buy-and-hold

> **SCOPING ONLY.** This brief designs the road; it builds nothing. No key
> handling, no exchange calls, no config changes, no git. The assistant /
> orchestrator NEVER executes a trade or moves funds. The operator arms;
> the system executes within armed caps. Every arming step is an explicit,
> auditable operator action — the agent can never self-arm.

## The product-boundary fact this brief consciously crosses

`spec/product.md` § Non-goals and § Project scope boundary state that real-money
execution, exchange API keys, and withdrawals are **out of scope for THIS
project** and become a "follow-up project" after the v3 continuous-paper
terminus. The operator has now ratified a **"Live-money path scoping"**
direction (2026-06-12). This brief IS the front edge of that follow-up program.
It does **not** silently overwrite the product boundary — it scopes the deliberate,
gated, human-armed extension past it, and names exactly what the architect must
amend in `product.md`/`architecture.md` before any code lands (Q1 below). Until
that amendment is ratified, `mode = "live"` stays rejected at config parse
(`config.rs:660-668`) and this is the correct default.

## What "live-money execution" actually means here — execute the PASSIVE baseline

Per the terminal verdict (`product.md` 2026-06-08, operator-ratified) and
`spec/runbooks/passive-baseline.md`, the ratified product ships the **passive
buy-and-hold baseline**, not the SMA crossover and not any active strategy. The
active-strategy research program is **concluded** (all three reachable channels
FRAGILE). Therefore the live-execution surface this program must serve is the
operational definition of the passive baseline, which is **radically simpler**
than the per-bar active loop:

| Property          | Passive baseline (from `passive-baseline.md`)                                        |
|-------------------|--------------------------------------------------------------------------------------|
| Strategy          | Buy-and-hold on the configured universe — **no signals, no per-bar decision**.       |
| Universe          | Config-driven (`data.sources.*`); program sample was 10 large-cap USDT perps/spot.   |
| Position rule     | Hold the universe; equal-weight initial allocation (operator-confirmed default).      |
| Rebalance cadence | **Monthly, equal-weight** (operator-ratified 2026-06-08). Quarterly / never are options. |
| Harness BH today  | A **pure buy-once-hold** (no rebalance ever); the monthly cadence is a forward proposal, **not yet backtested**. |

The single most important design consequence: **a buy-and-hold-with-monthly-
rebalance policy is a SCHEDULE-driven executor, not a per-bar signal loop.** It
acts at most ~12-13 times a year (1 inception buy + ~12 monthly rebalances on a
1-year horizon), not once per bar. This reframes everything downstream — see § F2
and Q3.

## What execution capability EXISTS today (file:line evidence)

The honest, evidenced answer: **there is no live-execution capability at all, by
design.** What exists:

| Capability                         | Status today | Evidence                                                                                  |
|------------------------------------|--------------|-------------------------------------------------------------------------------------------|
| Live exchange ORDER client         | **ABSENT**   | No `place_order`/`new_order`/HMAC/`api_secret`/signature anywhere in `crates/` or `src/` (grep clean). |
| `ExecRouter` trait                 | Stub only    | `crates/exec/src/router.rs:19` trait; `PaperExecRouter::submit` returns `UnsupportedMode("not yet wired (T24)")` (router.rs:28-32). It is essentially unused — paper fills do NOT flow through it. |
| Paper fill path                    | In-process   | Real paper fills come from `PaperEngine::step` (seed `0x00C0_FFEE`) called directly in `spawn_trading_loop` (ADR-0053 D6 / runtime.rs:958+), **not** via `ExecRouter`. |
| Binance market-data client         | Read-only    | `crates/data/src/binance.rs` — `reqwest::get` UNauthenticated (binance.rs:207) + WS klines/trades. NO auth, NO order endpoint, NO account/balance endpoint. |
| Account / balance / position read  | **ABSENT**   | The only `account_balance` (`crates/audit/src/query.rs:1654`) is a LEDGER account query, not an exchange call. |
| `Mode::Live`                       | **ABSENT**   | Enum is `{Research, Paper}` (config.rs:525-528); `mode = "live"` is rejected at parse time (config.rs:660-668) with a guard test (`t12_mode_live_is_rejected`). |
| Reconciler                         | In-process   | `equity() = cash + position_qty * last_mark` (reconciler.rs:58-60) — pure in-process state. Imbalance check (reconciler.rs:222-229) compares in-process equity to its OWN prior value, not to any exchange; its own doc admits "full implementation needs ledger query". |
| Kill switch                        | **SOLID**    | `crates/agent/src/kill_switch.rs` — sticky, CAS-guarded (kill_switch.rs:274-313), `.halt` file watcher (1 s poll), heartbeat monitor, audit dual-write + incident-report spawn on trip. Broadcasts `AgentMode::Halted`. **Caveat:** it broadcasts a mode change; it does NOT itself cancel/flatten live orders (nothing reads `Halted` to call an exchange cancel today — because there is no exchange). |
| Secret boundary precedent          | **EXISTS**   | LLM keys live in git-ignored `config/agent.toml.local` (`config/agent.toml:92-100`, `config/agent.toml.local.example`); committed config carries only an `api-key` placeholder. This is the boundary the live-key wiring extends. |

The unified per-bar loop (ADR-0053) was explicitly built to "carry to the
live-money path" — but it carries the *equity/persist topology*, not an
execution client. And see Q3: the **passive policy does not obviously want a
per-bar loop at all.**

## Goal

Scope the road from "paper-terminal research codebase" to "operator-armed,
capped, auditable live execution of the passive buy-and-hold baseline on
Binance" — cut into 2-4 shippable features with human gates at every arming
step, with the riskiest design decisions surfaced as architect questions.

## Non-goals (held hard)

- **No active strategy ever goes live without a NEW ratified research verdict.**
  The concluded research program (`product.md` 2026-06-08) found all reachable
  channels FRAGILE; live execution is the PASSIVE baseline only. Any future
  active-live request is a fresh program with its own robustness gate.
- **No withdrawals, no fund transfers, ever.** The live client places spot BUY
  orders to build the baseline and (rebalance) orders to maintain weights. It
  never moves funds off the exchange.
- **No HFT, no market making, no leverage, no derivatives execution.** Spot
  buy-and-hold only. (Perps were signal-only even in the research program.)
- **No multi-venue.** Binance only for F1-F3; multi-venue is a later program.
- **No secrets in git, ever** (CLAUDE.md non-negotiable). The repo never sees a
  live key; the operator provisions out-of-band into a git-ignored local file
  or the host secret store.
- **No order-entry UI.** The cockpit never writes orders or config (`product.md`
  § What stays out of the cockpit IA). Arming is config + a runtime guard +
  audit, surfaced read-only in the cockpit.

## Proposed feature cut (the phasing)

This is an **L/XL multi-feature program**, not one feature. Proposed cut into
three shippable features, each with its own `[[req]]` row, sequenced with
human gates between:

### F1 — Live exec client + real-exchange reconciliation (`REQ-LIVE-PASSIVE-EXEC-F1-001`) — effort **L**

The execution substrate. **Testnet-first.** Builds a real (authenticated)
Binance order client behind a `LiveExecRouter` trait, plus account/balance/
position reconciliation against the REAL exchange. Ships pointed at **Binance
Spot TESTNET only** — no mainnet path armed in F1. This is where every external
I/O-behind-a-trait, error/retry, lot/precision, and key-boundary decision lands.

**In scope:** `LiveExecRouter` trait + `BinanceSpotExecClient` (signed REST:
place market/limit order, query order status, cancel); account snapshot reader
(real balances + positions); the reconciler's real-exchange divergence check
(replace the self-referential heuristic — reconciler.rs:222-229); key-loading
boundary (env / git-ignored local / host secret store — operator provisions; repo
never sees keys); exchange filter ingestion (`LOT_SIZE`, `MIN_NOTIONAL`,
`PRICE_FILTER` step/tick) so orders are pre-validated client-side; error/retry/
idempotency semantics (client order IDs, partial-fill handling, rate-limit
backoff). Testnet rehearsal proof of the full pipeline (fake money).

### F2 — `PassiveBaseline` policy + arming mechanism (`REQ-LIVE-PASSIVE-EXEC-F2-001`) — effort **M-L**

The schedule-driven passive executor and the human-arming guard. Adds a
`PassiveBaseline` allocation policy (initial equal-weight allocation +
monthly-rebalance schedule + zero signals) and the **arming mechanism** that
makes live execution impossible without an explicit operator action.

**In scope:** the `PassiveBaseline` policy (see § What strategy/ needs); the
`Mode::Live` enum variant + un-rejecting `mode = "live"` at parse (gated, see
Q1); the **multi-factor arming guard** (config flag + operator-armed file/token
+ max-notional cap, ALL required — see § Arming mechanism); the
operator-side-pending-ledger arming audit trail; wiring the schedule-driven
executor to F1's `LiveExecRouter`; the baseline-equity-divergence e2e gate (see
Q4 — likely APPLIES). F2 ships pointed at testnet (arming guard proven on
testnet); it does NOT itself arm mainnet.

### F3 — Canary runbook + safety drills (`REQ-LIVE-PASSIVE-EXEC-F3-001`) — effort **M**

The operational close. The staged-rollout runbook (testnet → live canary →
scale-up), the kill-switch-halts-live-orders drill, the exec-side caps
enforcement proof, and the live monitoring backbone wiring (the durable equity
history just shipped). This is the feature that produces the human-verification
recipes the operator runs before/while arming real capital.

**In scope:** `spec/runbooks/live-canary.md` (the arming ladder with explicit
operator gates); the kill-switch live-order-halt drill (prove `.halt` cancels
open + blocks new live orders — closes the kill-switch caveat above); exec-side
max-notional / max-position cap enforcement tests (caps enforced at the
`LiveExecRouter`, NOT only strategy-side); reconciler-vs-real-exchange divergence
→ halt drill; durable-equity-history (ADR-0052) as the live monitoring backbone +
alerting-gap inventory. The canary cap-armed-tiny-capital recipe.

> **Sequencing + gates.** F1 → (human gate: testnet rehearsal passes) → F2 →
> (human gate: arming guard proven on testnet) → F3 → (human gate: operator
> arms live canary with tiny cap) → scale-up (separate operator ratification).
> F1 and F2 have a hard dependency (F2 executes through F1's router). F3 can
> start its runbook/drill scaffolding in parallel with F2 but gates on both.

## The capability gap map (what F1/F2 must build)

1. **Exchange order client (testnet-capable).** A signed Binance Spot REST
   client (HMAC-SHA256 over the query string; `X-MBX-APIKEY` header) supporting
   `POST /api/v3/order` (market + limit), `GET /api/v3/order` (status),
   `DELETE /api/v3/order` (cancel), behind a `LiveExecRouter` trait so tests fake
   it. Testnet base URL (`https://testnet.binance.vision`) vs mainnet
   (`https://api.binance.com`) is a config/secret-bound switch, never hard-coded.
   Reuses `crates/data/src/binance.rs`'s `reqwest`/connect plumbing patterns but
   is a SEPARATE authenticated client (the market-data feed stays unauthenticated).

2. **Account / balance / position reconciliation against the REAL exchange.**
   `GET /api/v3/account` (signed) for real balances; the reconciler's imbalance
   check is rebuilt to compare ledger/in-process state against the **exchange-
   reported** balances + positions (replacing reconciler.rs:222-229's self-
   reference). Divergence beyond tolerance → kill switch (`LedgerImbalance`
   already exists as a `HaltReason`, kill_switch.rs:51).

3. **Key management (design the boundary; repo never sees keys).** The operator
   provisions `BINANCE_API_KEY` / `BINANCE_API_SECRET` out-of-band into EITHER
   env vars OR a git-ignored `config/agent.toml.local` (the existing LLM-key
   precedent) OR a host secret store. The code reads from a `SecretSource` trait
   (env-backed default; testable). Keys are NEVER logged, NEVER written to the
   audit ledger, NEVER committed. F1 designs and tests this boundary with FAKE
   testnet keys provided by the operator out-of-band.

4. **Order-type minimal set for passive.** MARKET buy is sufficient for inception
   allocation and monthly rebalance on a hold strategy (simplicity > price
   improvement for a low-turnover baseline). LIMIT is an operator option for slippage
   control on larger notionals (see Q5). Size precision/lot rules: ingest
   `LOT_SIZE` (`stepSize`, `minQty`), `MIN_NOTIONAL`, `PRICE_FILTER` (`tickSize`)
   from `GET /api/v3/exchangeInfo` and round/validate every order client-side
   BEFORE submit — an under-min order must fail fast, not at the exchange.

5. **Error / retry semantics.** Client order IDs (`newClientOrderId`) for
   idempotency on retry; partial-fill accounting; rate-limit (HTTP 429 / `-1003`)
   exponential backoff with a hard cap; on ambiguous timeout, query order status
   before any retry (never blind-resubmit a possibly-filled order). A failed
   order after N retries → log + halt, never silent.

## Arming mechanism sketch (the agent can NEVER self-arm)

Design principle: **defense in depth — multiple independent operator actions
must ALL be present for a single live order to leave the process.** No single
flag, and certainly no agent code path, can arm live execution.

```
Live order leaves the process  ⇔  ALL of:
  (1) config:  mode = "live"            (operator hand-edits config/agent.toml;
                                         un-rejected only post-Q1 amendment)
  (2) arm-file: ./.live-armed present   (operator creates out-of-band, like .halt
                                         in reverse; absence = disarmed; mirrors
                                         the kill-switch .halt convention)
  (3) cap:     [live].max_notional_usdt set AND order ≤ cap   (exec-side enforced
                                         at LiveExecRouter, NOT strategy-side)
  (4) secret:  BINANCE_API_KEY/SECRET present from the SecretSource
                                         (operator provisions out-of-band)
  (5) NOT halted: kill switch not tripped (.halt absent)
```

- **The arm-file is the human gate.** Like `.halt` trips the kill switch, `.live-armed`
  arms live execution — but its presence is necessary-not-sufficient (the other four
  conditions also gate). It is git-ignored; the operator creates it deliberately;
  the agent never creates it. Restart with arm-file absent = disarmed (fail-safe
  default).
- **Audit trail.** Every transition (armed / disarmed / order-blocked-by-cap /
  order-blocked-by-disarm) writes a row to the audit ledger via the existing
  `strategy_events` + memo dual-write seam (the kill-switch precedent,
  kill_switch.rs:287-301). The operator-side-pending-ledger convention records
  "operator armed live canary, cap=$X, at T" as an explicit human-action row.
- **The orchestrator/assistant never arms and never executes.** Stated plainly
  in the runbook (F3): arming is an operator-only physical action (edit config +
  create arm-file + provision keys); the system executes within the armed cap;
  the assistant's role ends at producing the recipe.
- **Fail-safe at every boundary.** Missing cap → no live orders. Missing keys →
  no live orders. Arm-file absent → no live orders. `.halt` present → no live
  orders + cancel open. Default config (`mode = "research"`, no arm-file) is
  fully disarmed.

## Safety hardening inventory (F3)

| Hardening                         | Today                                  | F3 must add                                                              |
|-----------------------------------|----------------------------------------|-------------------------------------------------------------------------|
| Kill switch halts LIVE orders     | Broadcasts `Halted`; nothing cancels orders (no exchange) | DRILL: `.halt` → `LiveExecRouter` cancels all open + rejects new live orders; prove it. |
| Max-notional / max-position caps  | `risk.per_symbol_exposure_cap` strategy-side only | EXEC-SIDE cap at `LiveExecRouter` — every order checked vs `[live].max_notional_usdt` / max-position regardless of what strategy/sizer asked. Defense-in-depth, not duplication. |
| Reconciler vs REAL exchange       | Self-referential heuristic (reconciler.rs:222-229) | Compare against `GET /api/v3/account`; divergence > tol → `HaltReason::LedgerImbalance`. |
| Durable equity monitoring         | ADR-0052 store shipped (paper)         | Wire `mode = "live"` as a writer (`build_snapshot_row` mode label `"live"`); live equity is the monitoring backbone. |
| Alerting                          | Cockpit `AgentMode::Halted` + incident report on trip | INVENTORY the gaps: no push/pager today; F3 names what live-money needs (at minimum: halt → incident report already wired; assess whether a heartbeat-to-operator channel is required for unattended live). |

## What the passive policy needs from `strategy/`

The `Strategy` trait is a single `on_bar(&mut self, bar: &Bar) -> Vec<Signal>`
(`crates/strategy/src/traits.rs:8-10`). A precedent for a trivial allocate-and-hold
strategy already exists: `crates/strategy/src/cash_hold.rs`. **But the per-bar
`on_bar` shape is a poor fit for a schedule-driven hold** (see Q3) — a passive
baseline acts on a calendar (inception + monthly), not per bar. Two shapes:

- **(a) `PassiveBaseline` as a schedule-aware policy** (recommended, see Q3) —
  not a per-bar signal generator; a small policy that emits an allocation
  intent at inception and on each monthly boundary, and emits nothing otherwise.
  This likely wants a **new seam** (a `RebalanceSchedule` + an allocator) rather
  than forcing the per-bar `Strategy` trait. Sizing: ~150-250 LoC + the schedule.
- **(b) `PassiveBaseline` jammed into `on_bar`** — returns signals only on the
  first bar and on bars crossing a month boundary, empty otherwise. Cheaper
  (~80 LoC, reuses the existing loop + registry) but couples a calendar policy to
  a bar-rate trait and re-introduces the per-bar loop the policy does not need.

**Baseline-equity-divergence gate (CLAUDE.md non-negotiable):** a sizing
decision DOES exist (initial allocation weights + rebalance trades), so the gate
**likely APPLIES** — do NOT rubber-stamp N/A. The honest read (Q4): the divergence
to assert is "armed live/testnet equity tracks the held-universe value and DIVERGES
from a flat do-nothing baseline once the inception allocation executes" — an e2e
test that the allocation actually moved capital into positions (the exact noop-class
the gate guards against: weights computed but orders never sent). Architect rules
APPLIES-or-N/A in F2 with reasoning, per the v3-vol-overlay precedent.

## Open questions for the architect

Each has a recommended default. **Q1 is the riskiest** (it is the
product-boundary + safety-boundary crossing).

- **Q1 (RISKIEST) — un-rejecting `mode = "live"`: where does the product/safety
  boundary move, and what is the minimum amendment?** Today `mode = "live"` is
  rejected at parse (config.rs:660-668) and `product.md` § Non-goals forbids
  real-money execution. **Recommended default: (a) keep the parse-rejection
  until a ratified `product.md`/`architecture.md` amendment lands that (i)
  redefines the scope boundary as "live = passive-baseline-only, Binance-only,
  spot-only, capped, operator-armed", (ii) adds a `Mode::Live` ADR with the
  arming-mechanism contract, and (iii) the F2 arming guard is the ONLY thing
  that flips the rejection — and even then live orders need all 5 arming
  conditions.** This is the durable choice: it makes the boundary-crossing an
  explicit, reviewed, ratified act rather than a quiet enum addition.
  *If-budget-tightens fallback:* (b) add `Mode::Live` now but keep it
  behaviorally inert (parses, but the arming guard rejects ALL live orders until
  F2/F3 land) — cheaper to wire but risks a half-armed mode existing before the
  safety rails do; only acceptable if F2/F3 are committed same-program.

- **Q2 — testnet vs mainnet client topology.** One `BinanceSpotExecClient`
  parameterized by base-URL+keys (testnet vs mainnet is config/secret), OR two
  distinct types? **Recommended default: (a) one client, URL+keys injected** —
  testnet and mainnet differ only in endpoint + credentials; one code path means
  the mainnet path is exercised by every testnet rehearsal (no "tested testnet,
  shipped untested mainnet" gap). The arming guard, not the client type, is what
  gates mainnet.

- **Q3 — does the passive policy want the ADR-0053 per-bar loop, or a
  schedule-driven executor?** ADR-0053's unified loop carries to live-money, but
  a monthly-rebalance hold acts ~12×/year, not per bar. **Recommended default:
  (a) a schedule-driven executor for the passive baseline, NOT the per-bar
  loop** — the per-bar loop is the right home for the (retired) active strategies;
  forcing a calendar policy through a bar-rate loop is the wrong fit. Reuse the
  loop's equity/persist topology (mark-to-market per bar for monitoring) but
  drive ORDER emission from a `RebalanceSchedule`, not `on_bar`. The architect
  should assess honestly whether the equity-monitoring half of the loop can be
  decoupled from the order-emission half. *Fallback:* (b) reuse the per-bar loop
  with a passive policy that no-ops except on rebalance bars — cheaper, but
  couples the calendar policy to bar arrival and inherits a loop the baseline
  doesn't conceptually want.

- **Q4 — does the baseline-equity-divergence gate apply to F2?** **Recommended
  default: YES, it applies** — a sizing decision exists (allocation weights +
  rebalance trades), so per the v3-vol-overlay precedent the architect records
  APPLIES with an e2e divergence proof (armed equity diverges from flat
  do-nothing once allocation executes; orders actually sent), NOT N/A. The only
  argument for N/A is "buy-and-hold has no per-bar signal" — but the inception
  allocation IS the decision the gate exists to verify got applied.

- **Q5 — order type for the live baseline: MARKET or LIMIT?** **Recommended
  default: (a) MARKET for inception + monthly rebalance** — a low-turnover hold
  values fill-certainty + simplicity over price improvement; MARKET avoids
  unfilled-limit complexity on a strategy that must hold the universe. LIMIT is
  a named operator option for slippage control on large notionals; ship MARKET
  first, add LIMIT only if canary slippage proves material.

- **Q6 — secret source: env var, git-ignored local file, or host secret store?**
  **Recommended default: (a) a `SecretSource` trait with an env-var-backed
  default, AND support for the existing git-ignored `config/agent.toml.local`
  precedent** — env vars are the cleanest for CI/host secret-store injection and
  never touch disk in the repo; the local-file path reuses the proven LLM-key
  boundary for operators who prefer a file. Both are out-of-band operator
  actions; the repo ships neither keys nor a non-placeholder config.

## Effort + phasing summary

| Feature | Scope                                            | Effort | Gates before it can arm anything                          |
|---------|--------------------------------------------------|--------|-----------------------------------------------------------|
| F1      | Live exec client + real-exchange reconciliation  | **L**  | Ships testnet-only; no mainnet path armed.                |
| F2      | `PassiveBaseline` policy + arming mechanism      | **M-L**| Arming guard proven on testnet; does NOT arm mainnet.     |
| F3      | Canary runbook + safety drills                   | **M**  | Operator arms live canary with tiny cap (explicit action).|

Total program: **L/XL.** Scale-up past the canary cap is a separate operator
ratification, not a feature.

## Assumptions (challenge these)

- A1: "Live-money path scoping" ratifies SCOPING this road, not building it. No
  code, no keys, no config, no git in this brief. (Stated in the operator brief.)
- A2: Binance Spot is the target venue (consistent with the research program's
  Binance-centricity and `data.sources.binance`). Binance Spot Testnet
  (`testnet.binance.vision`) is the rehearsal environment.
- A3: The passive baseline is monthly-equal-weight (operator-ratified
  2026-06-08, `passive-baseline.md` changelog) — F2 implements that cadence; the
  harness's pure-buy-once-hold is the research control, a different artifact.
- A4: The operator provisions all keys out-of-band; the repo never sees a live
  key. Testnet keys for F1 rehearsal are also operator-provisioned out-of-band.
- A5: This program does NOT reopen the active-strategy research verdict. Live =
  passive only. Any active-live request is a fresh ratified program.

## Changelog

- 2026-06-12 (analyst): created. Scoped the live-money passive-execution road
  into three shippable features (F1 testnet exec client + real-exchange
  reconciliation; F2 `PassiveBaseline` policy + 5-condition arming mechanism; F3
  canary runbook + safety drills). Anchored on the terminal SHIP-PASSIVE verdict
  (`product.md` 2026-06-08) and `passive-baseline.md` — live execution is the
  passive buy-and-hold baseline, never an active strategy. Mapped the current
  capability with file:line evidence: NO live exec client / NO exchange
  account-read / NO `Mode::Live` exist today (all by design — `mode = "live"`
  rejected at config.rs:660-668); the kill switch + reconciler exist but the
  reconciler trusts in-process state and the kill switch broadcasts `Halted`
  without cancelling (no exchange to cancel against). Designed the
  agent-can-never-self-arm mechanism (5 independent operator-gated conditions:
  config mode + arm-file + exec-side cap + secret presence + not-halted), the
  exec-side cap enforcement, and the real-exchange reconciliation. Surfaced 6
  architect questions with recommended defaults; Q1 (the product/safety boundary
  amendment that un-rejects `mode = "live"`) flagged as riskiest. Ruled the
  baseline-equity-divergence gate likely APPLIES to F2 (a sizing decision exists)
  — NOT rubber-stamped N/A. Created `REQ-LIVE-PASSIVE-EXEC-F1/F2/F3-001` rows
  (state proposed). Scoping only — no code, no keys, no config, no git.
