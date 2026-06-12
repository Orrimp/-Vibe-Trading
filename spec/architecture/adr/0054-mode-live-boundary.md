---
adr: 0054
title: Mode::Live boundary — passive-only, Binance-spot-only, exec-side-capped, operator-armed, kill-switch-supreme; parse-rejection retained until F2 lands
status: proposed
date: 2026-06-12
supersedes: none
superseded-by: none
---

# ADR-0054: The `Mode::Live` boundary — what live execution is permitted to be, and the contract that gates it

## Context

The operator ratified a **"Live-money path scoping"** direction on
2026-06-12 (`spec/live-passive-execution-readiness/feature.md`). This is
the front edge of the follow-up program `spec/product.md` § Project scope
boundary names — the deliberate, gated extension past the paper-terminal
finish line. This ADR exists to make that boundary-crossing an
**explicit, reviewed, ratified architectural act** rather than a quiet
enum addition, and to fix — before any code lands — exactly what
`Mode::Live` is *permitted to be* and the contract that gates a single
live order leaving the process.

**Verified load-bearing facts** (this design rests on these — each
re-checked against the tree at the cited line, not the analyst's brief):

1. **No live execution capability exists today, by design.** Grep is
   clean for `place_order` / `new_order` / HMAC / `api_secret` /
   signature across `crates/` and `src/`. `ExecRouter` is a stub:
   `PaperExecRouter::submit` returns `ExecError::UnsupportedMode("...not
   yet wired (T24)")` (`crates/exec/src/router.rs:19-33`) and is unused —
   real paper fills come from `PaperEngine::step` (seed `0x00C0_FFEE`)
   called directly in the unified trading loop (ADR-0053 D6), never via
   `ExecRouter`.
2. **`Mode` is `{Research, Paper}`** (`crates/agent/src/config.rs:525-528`).
   `mode = "live"` is rejected at parse by a deliberate two-step parse:
   `from_toml_str` reads the raw TOML, and if `mode` case-insensitively
   equals `"live"` returns `ConfigError::UnsupportedMode` BEFORE the enum
   would silently fail (`config.rs:660-668`), guarded by the test
   `t12_mode_live_is_rejected`. This parse-rejection is the current
   correct default and the load-bearing boundary this ADR governs.
3. **The market-data client is read-only / unauthenticated.**
   `BinanceFeed::exchange_info` uses `reqwest::get` with no auth header
   (`crates/data/src/binance.rs:201-213`); `BinanceFeed::production()`
   hard-codes the mainnet URLs (`binance.rs:128-133`). There is NO order
   endpoint, NO `GET /api/v3/account`, NO signing. The feed already
   parses exchange-info filters but only read-only.
4. **The reconciler trusts in-process state.** `ReconcilerState::equity()
   = cash + position_qty * last_mark` (`crates/agent/src/reconciler.rs:58-60`);
   the imbalance check compares `current_equity` to its OWN prior value
   `last_equity` (`reconciler.rs:222-229`) and its own comment admits
   "full implementation needs ledger query". It has no exchange truth
   source.
5. **The kill switch is solid but broadcast-only.**
   `KillSwitch::trip` is sticky + CAS-guarded (only the first caller wins
   via `tripped.swap(true, SeqCst)`, `kill_switch.rs:274-313`), with a
   `.halt` file watcher (1 s poll), heartbeat monitor, audit dual-write
   (`audit::journal::kill_switch_tripped`), and incident-report spawn. It
   broadcasts `AgentMode::Halted { reason }` — but **nothing cancels or
   flattens orders**, because there is no exchange to cancel against.
   `HaltReason::LedgerImbalance` already exists (`kill_switch.rs:52`).
6. **A secret boundary precedent exists.** LLM provider keys live in a
   git-ignored `config/agent.toml.local` merged at load
   (`config.rs:612-651`, `merge_llm_local_overlay`); the committed config
   carries only placeholders. This is the boundary the live-key wiring
   extends — never a new mechanism.

The analyst surfaced six questions; their resolutions (Q1–Q6) are
recorded in the feature `## Architecture` section. This ADR carries the
**normative core**: the five invariants, the arming contract, the
permitted shape of `Mode::Live`, and the proposed `product.md` amendment
text so ratification is one operator decision, not a drafting exercise.

## Decision

### D1 — `Mode::Live` exists ONLY in a hard-bounded shape; everything else is a non-goal

When the parse-rejection (D5) is lifted, `Mode::Live` is permitted to
mean **only** the conjunction of all of:

- **Passive-only.** It executes the `PassiveBaseline` policy
  (equal-weight inception allocation + monthly rebalance + ZERO signals;
  `spec/runbooks/passive-baseline.md`, operator-ratified 2026-06-08). It
  executes **no active strategy** — not `SmaCrossover`, not momentum, not
  any robustness-program arm. The active-strategy research program
  concluded FRAGILE across all three reachable channels
  (`product.md` 2026-06-08); a live active strategy requires a **fresh
  ratified research program** with its own robustness gate (D6).
- **Binance-spot-only.** One venue, spot only. No perps (perps were
  signal-only even in research), no leverage, no derivatives, no margin,
  no multi-venue.
- **Exec-side-capped.** Every order is checked against
  `[live].max_notional_usdt` **at the `LiveExecRouter`**, not only at the
  strategy/sizer. Defense in depth: the cap holds regardless of what the
  policy asked.
- **Operator-armed.** Live orders are impossible without the explicit,
  out-of-band, multi-factor operator arming of D2. The agent can never
  self-arm; the assistant/orchestrator never arms and never executes.
- **Kill-switch-supreme.** A tripped kill switch (`.halt` present or any
  `HaltReason`) is absolute: it cancels open live orders and rejects new
  ones, overriding every other condition.
- **No withdrawals, ever.** The live client places spot BUY orders to
  build and maintain the baseline and never moves funds off the exchange.
  No transfer endpoint is wired.

### D2 — The 5-condition arming contract (the normative core; the agent can NEVER self-arm)

**A live order leaves the process if and only if ALL FIVE of the
following independent operator-established conditions hold simultaneously:**

```
Live order leaves the process  ⇔  ALL of:
  (1) config:   mode = "live"                  — operator hand-edits config/agent.toml
                                                 (un-rejected only post-D5/F2 amendment)
  (2) arm-file: ./.live-armed present          — operator creates out-of-band; mirrors the
                                                 kill-switch .halt convention in reverse;
                                                 git-ignored; the agent NEVER creates it;
                                                 restart with it absent = disarmed (fail-safe)
  (3) cap:      [live].max_notional_usdt set    — AND the order's notional ≤ cap, enforced
                                                 EXEC-SIDE at the LiveExecRouter (not strategy-side)
  (4) secret:   BINANCE_API_KEY / _SECRET       — present from the SecretSource (operator
                                                 provisions out-of-band; repo never sees keys)
  (5) NOT halted: kill switch not tripped       — .halt absent AND no HaltReason live
```

Defense in depth: **no single flag, and no agent code path, can arm live
execution.** Each condition is a separate, independently-revocable
operator action. The five are checked in a **fixed fail-fast order**
(cheapest/most-decisive first; see feature `## Architecture` A4): (5)
not-halted → (1) mode → (2) arm-file → (4) secret presence → (3)
per-order cap. The first failing condition short-circuits to **zero
orders leave** and writes one audit row naming the blocking condition.

### D3 — The five invariants are normative and binding (verbatim)

These five statements are binding law for every feature, task, and
review under this program. They are reproduced verbatim wherever the
program is implemented:

> 1. **No secrets in git, ever.** The `SecretSource` design makes the
>    safe path the only path — the repo ships neither keys nor a
>    non-placeholder config; keys are read from env or a git-ignored
>    local file or a host secret store, and are NEVER logged, NEVER
>    written to the audit ledger, NEVER committed.
> 2. **Money is `Decimal` / `Money<Usdt>`, never `f64`.** Every notional,
>    cap, balance, and reconciliation quantity is `rust_decimal::Decimal`
>    (ADR-0003). P&L aggregation reconciles exact-cent, no tolerance.
> 3. **Every external I/O is behind a trait.** The exchange order client,
>    the account reader, and the secret source are each behind a trait so
>    tests fake them; no test ever touches a real exchange or a real key.
> 4. **The operator arms; the agent never self-arms; the assistant never
>    executes.** Arming is an operator-only physical action (edit config +
>    create `.live-armed` + provision keys). The agent has no code path
>    that creates the arm-file or sets the live cap. The
>    assistant/orchestrator's role ends at producing the recipe; it never
>    places an order or moves funds.
> 5. **The kill switch is supreme.** A tripped `.halt` (or any
>    `HaltReason`) cancels open live orders and rejects new live orders,
>    overriding mode, arm-file, cap, and secret presence. Halted is a
>    terminal disarmed state until the operator removes `.halt` and
>    restarts (the existing sticky-halt R7.3 contract).

### D4 — The baseline-equity-divergence gate APPLIES to F2 (Q4), and the tester gates on it

Per the CLAUDE.md non-negotiable and the `v3-volatility-forecaster-noop-fix`
precedent: the `PassiveBaseline` policy makes a **sizing decision** (the
equal-weight inception allocation + monthly rebalance trades), so the
baseline-equity-divergence gate **APPLIES — it is NOT N/A**. F2 ships an
e2e proof that armed (testnet) equity **diverges** from a flat
do-nothing baseline once the inception allocation executes — i.e. the
allocation actually moved capital into positions and the orders were
actually sent. This is the exact no-op class the gate guards against
(weights computed but orders never sent). This ruling is recorded here so
the tester gates on it explicitly and cannot rubber-stamp N/A.

### D5 — The parse-rejection is RETAINED until F2 lands; F2's arming guard is the ONLY thing that lifts it

`config.rs:660-668`'s `mode = "live"` parse-rejection **stays in force**
until BOTH (a) the operator ratifies the `product.md` amendment in D7,
AND (b) F2's arming guard is implemented. Even then, lifting the
rejection only makes `Mode::Live` *parseable*; a live order still
requires all five D2 conditions. There is no intermediate state in which
`Mode::Live` parses but the arming guard does not yet exist — the
un-rejection and the guard land in the same feature (F2), so a half-armed
mode can never exist on `main`. Until F2, `mode = "live"` must continue
to fail at parse with `ConfigError::UnsupportedMode` and the
`t12_mode_live_is_rejected` guard test stays green.

### D6 — Explicit non-goal: no active strategy goes live without a fresh ratified research program

`Mode::Live` executing an **active** strategy is an explicit non-goal of
this program. The concluded research verdict (FRAGILE across price /
positioning / on-chain, `product.md` 2026-06-08) is not reopened here.
Any future active-live request is a **new program** with its own
pre-registered robustness gate and its own ratification — it is not a
continuation of this scoping and not unlocked by any artifact this
program ships. Live = passive only.

### D7 — Proposed `product.md` amendment (PROPOSED text for operator ratification; the operator ratifies — the architect does NOT edit product.md)

This ADR is `status: proposed`. The boundary does not move until the
operator ratifies the amendment below. The exact wording is carried here
so ratification is a single decision, not a drafting exercise. On
ratification, the analyst (product.md owner) applies these two edits and
this ADR moves to `accepted`.

**Proposed edit 1 — `product.md` § Non-goals**, replace the bullet
beginning "**Real-money execution, KYC, exchange API keys,
withdrawals.**" with:

> - **Real-money execution beyond the passive baseline, KYC,
>   withdrawals.** Out of scope for this project. The ONE ratified
>   exception (2026-06-12, ADR-0054) is **operator-armed live execution
>   of the passive buy-and-hold baseline on Binance spot only**, capped
>   exec-side, behind a 5-condition arming contract the agent can never
>   self-satisfy. No active strategy, no leverage, no derivatives, no
>   multi-venue, no withdrawals. Exchange API keys are operator-provisioned
>   out-of-band and never enter the repo. All other real-money execution
>   (active strategies, other venues, custody) remains a follow-up project.

**Proposed edit 2 — `product.md` § Project scope boundary**, append after
the "What it does **not** ship" list:

> **Ratified boundary exception (2026-06-12, ADR-0054).** A single,
> tightly-bounded extension is now in scope: **operator-armed, exec-side-
> capped live execution of the passive buy-and-hold baseline on Binance
> spot**. It is passive-only (no active strategy without a fresh ratified
> research program), Binance-spot-only, withdrawal-free, and impossible
> without all five independent operator arming conditions (mode +
> arm-file + exec-side cap + secret presence + not-halted). The
> kill switch is supreme. This narrows — it does not erase — the
> paper-terminal boundary: everything outside this exception remains a
> follow-up project.

## Alternatives considered

- **(Q1-b) Add `Mode::Live` now but keep it behaviorally inert** (parses,
  but the arming guard rejects all live orders until F2/F3 land).
  **Rejected as the primary.** Cheaper to wire, but it risks a half-armed
  mode existing on `main` before the safety rails do — a live enum that
  parses with no guard behind it is exactly the boundary-erosion this ADR
  exists to prevent. Only acceptable as an if-budget-tightens fallback if
  F2/F3 are committed same-program; D5 instead keeps the un-rejection and
  the guard atomic in F2.
- **A single flag (e.g. just `mode = "live"`) arms live execution.**
  **Rejected.** A single point of failure for real money. Defense in
  depth (D2's five independent conditions) is the whole point: a
  misconfiguration, a stray commit, or an agent bug must not be able to
  arm by itself.
- **Cap enforced strategy-side only** (reuse `risk.per_symbol_exposure_cap`).
  **Rejected** for the live path. The strategy-side cap remains, but the
  load-bearing cap is **exec-side at the `LiveExecRouter`** so it holds
  regardless of a sizer bug — defense in depth, not duplication.
- **Mainnet as a distinct client type from testnet.** **Rejected**
  (Q2=a): one `BinanceSpotExecClient` with URL+keys injected, so every
  testnet rehearsal exercises the exact mainnet code path. The arming
  guard — not the client type — gates mainnet. (Recorded in feature
  `## Architecture` A1.)
- **Edit `product.md` in this pass.** **Rejected by contract.** The
  analyst owns `product.md`; the operator ratifies the boundary move. The
  ADR carries the exact proposed wording (D7) so the architect proposes
  and the operator disposes — the boundary never moves silently
  (CLAUDE.md non-negotiable: "No silent divergence from
  `spec/architecture.md`" and its product-boundary analogue).

## Consequences

- **Positive.** The boundary-crossing is now an explicit, reviewed,
  ratified record. The five invariants are binding and reproduced
  verbatim wherever the program is built. A half-armed `Mode::Live` can
  never exist on `main` (D5 atomicity). The non-goal (active-live) is
  stated so no future feature quietly unlocks it. The proposed amendment
  is drafted, so operator ratification is one decision.
- **Negative / risk.** This ADR ratifies *intent*; the safety is only as
  real as F1/F2/F3's tests. Mitigation: every safety-critical task names
  its adversarial test (the arming-guard "any one condition missing ⇒
  zero orders leave" matrix, the kill-switch order-cancel drill, the
  exec-side cap-enforcement matrix, the reconciler-divergence→halt drill)
  in `tasks.md`, and D4 forces the divergence e2e proof on F2.
- **Scope held.** No code, no keys, no config, no git in this pass. The
  parse-rejection (D5) stays until F2. `Mode::Live` is not added in this
  pass — only its permitted shape and gating contract are fixed. No
  anchor SHA in `spec/anchors.toml` is touched (this ADR changes no
  hashed report body; it is anchor-neutral by construction).

Resolves feature `live-passive-execution-readiness` Q1 (D5/D7 — retain
parse-rejection, atomic un-rejection in F2, proposed amendment carried),
and records the normative core (D1/D2/D3/D6) that F1/F2/F3 build against.
Q2–Q6 resolutions live in the feature `## Architecture` section.

## Changelog
- 2026-06-12 (architect): initial proposal. `Mode::Live` bounded to
  passive-only / Binance-spot-only / exec-side-capped / operator-armed /
  kill-switch-supreme (D1); the 5-condition arming contract as the
  normative core (D2); the five binding invariants verbatim (D3); the
  baseline-equity-divergence gate APPLIES to F2 (D4); the parse-rejection
  retained until F2 lands atomically (D5); active-live an explicit
  non-goal (D6); the proposed product.md amendment carried for operator
  ratification (D7). Status `proposed` until the operator ratifies the D7
  amendment, at which point it moves to `accepted`.
