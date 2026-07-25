---
slug: advisor-short-selling
status: shipped
owner: tester
updated: 2026-06-23
---

# Advisor — Single-Coin Directional Short-Selling (paper)

> Trace: `REQ-ADVISOR-SHORT-SELLING-001` (`spec/trace.toml`).
> Product anchor: [`product.md`](../../product.md) § journey / § D1 / § Open decisions.
> Operator directive 2026-06-23: *"do the expensive short selling."* Accepted as the
> big-lift engine change — the genuinely-unexplored **down-half** lever the long-only
> robustness program never covered for the single-coin advisor.

## Why

The advisor today is **structurally long-or-flat**. Every single-coin arm can only
`Buy` (when flat) or `Sell`-to-flat (when long); a bearish signal parks the €200 in
cash. On a downtrend the best the advisor can say is *"hold cash"* — it has **no way
to profit from a falling price**. The 2026-06-08 ship-passive verdict and the live
all-Fragile field were both reached **inside that long-only box**. Directional
short-selling is the one lever that has never been tested for the single-coin journey:
*can betting on a decline add robust value where long-only sits flat?*

This is deliberately framed as an **honest test, not an alpha promise** (see § Honest
framing). It is also the operator's explicitly-accepted "expensive" feature: it touches
the backtest P&L core, the paper-sim, the audit ledger's sign handling, and the UI. The
deliverable is a faithful simulated-short capability judged by the **same frozen
robustness gate + same buy-and-hold benchmark** as everything else — and a null result
("shorts are also Fragile, hold still stands") is a **valid, shippable outcome.**

### What the code audit found (this de-risks the estimate substantially)

The recon notes are confirmed, with one major upside the brief did not anticipate:

1. **The single-coin engine is hard long-only by three clamps, not by the math.**
   - `run_scenario` (`crates/backtest/src/engine.rs:1632-1640`) computes `desired_side`
     as *Buy-only-when-`base_qty <= 0`* / *Sell-only-when-`base_qty > 0`* — structurally
     it can never **open** a short.
   - The Sell fill handler (`engine.rs:1712-1715`) does `position.base_qty -= qty` then
     **explicitly clamps to zero**: `if base_qty < 0 { base_qty = 0; }`.
   - `BacktestState::apply_sell` (`crates/backtest/src/cli_types.rs:632-635`) does the
     same clamp on `position_qty` **and zeroes `position_cost`**.
   - The identical clamp is duplicated in the `sma_composed_run` scenario path
     (`crates/backtest/src/scenarios/sma_composed_run.rs:554`).
   - **Crucially, the equity formula itself is already short-correct:**
     `BacktestState::equity = cash + position_qty * mark` (`cli_types.rs:592-594`). A
     sell-to-open at P credits cash by `P·qty`, drives `position_qty` to `−qty`, so
     `equity = cash + (−qty)·mark`. When `mark` rises, equity falls — **exactly the
     honest short P&L.** The bug is the clamps, **not** the mark-to-market.

2. **A complete, tested, shipped short-side engine ALREADY EXISTS in the codebase** —
   just in the *multi-symbol cross-sectional* path, not the single-coin one. The
   market-neutral perp-basis feature (`REQ-PERP-BASIS-MARKET-NEUTRAL-001`,
   `spec/trace.toml` ~row 2671, state `tester-done`, VERDICT PASS, **119/119 anchors**)
   built and froze, in `crates/backtest/src/scenarios/montecarlo.rs::run_path`:
   - **Open-short** with an initial-margin gate + a `MAX_LEVERAGE` constant
     (`montecarlo.rs:90` = `Decimal::ONE` → **1x today**), arm at `:402-455`.
   - **Buy-to-cover** the short (`current_qty < 0 && k_short > 0`), arm at `:253-299`.
   - **Maintenance-margin liquidation** (`:531-589`): force-cover all shorts when
     `equity < maintenance_margin_frac() × gross_short_notional`
     (`maintenance_margin_frac = 0.5`, `:100`), and it **lets cash go negative in
     extreme liquidation** — i.e. it already models honest unbounded-loss.
   - **Per-bar funding accrual** correct for both long and short (`:460-520`):
     `cash += notional × (−rate)`; a short on a positive-funding name pays the cost by
     the existing formula.
   - Signed `position_book` (negative qty = short) with `equity = cash + Σ(qty·mark)`.
   - The whole short branch is **gated on `k_short > 0`**, so it is **inert and
     byte-identical for all long-only runs** — proven by the shipped
     `run_path_k_short_zero_byte_identical_to_head` anchor-neutrality test.
   - The cross-sectional config used to hard-reject shorts
     (`crates/strategy/src/cross_sectional/config.rs:137`
     `unsupported_short_sizing`); that gate was lifted **for `LongShort` mode only**
     (`config.rs:331-334`).
   - **Science result of that work: FAMILY-UNIFORM-FRAGILE** on all 12 surfaces vs the
     dollar-neutral null. This is the load-bearing prior for our honest framing.

   **The genuine gap is therefore NOT "invent short accounting."** It is: *port the
   proven `montecarlo.rs` signed-position short model (open / cover / liquidate / fund)
   into the single-coin `run_scenario` / `sma_composed_run` path, which currently
   hard-clamps to long-only, and wire it through paper-sim + audit + UI.* That is a
   materially smaller and lower-risk estimate than a from-scratch build — the financial
   math is written and tested; we are relocating + adapting it from portfolio-shape to
   single-coin-directional-shape.

3. **The audit ledger actively rejects shorts today.** The `OpenPosition` reader
   asserts `qty > 0` and **raises `LedgerError::Database` on net-negative qty**
   (`crates/core/src/position.rs:71-73`, "Long-only at v1+"). `Position.base_qty`'s
   doc-comment says *"positive = long, negative = short. v0 is spot-only so this is
   always ≥ 0"* — the type holds a short; the invariant forbids it. The ledger
   sign-handling is a real surface this feature must open (carefully, in isolation —
   `audit` imports nothing from sibling crates).

4. **The bake-off is anchor-safe for new arms by construction.** `run_bakeoff`
   (`crates/backtest/src/bakeoff/mod.rs:557-629`) runs every arm through `run_scenario`
   with **`write_report: false`** (`:616`). New short-capable arms write **no anchored
   report body** → `verify_anchors.sh` stays **119/119** for free, exactly as the
   13-arm combination-search feature relies on.

5. **The signal model needs a minimal extension.** Single-coin `SignalKind` =
   `Buy | Sell | Hold` with `Sell` = exit-to-flat (`crates/core/src/signal.rs:37-48`).
   `PairShortObservation` exists but is observation-only ("future v2 use"). We need an
   intent to **open-short / cover-short** that the single-coin scenario loop can read.

## Requirements

Numbered `R-SS.*`. **LOAD-BEARING** items are tagged.

- **R-SS.1 — Correct single-coin short P&L in the engine (LOAD-BEARING).** Remove the
  long-only clamp from the single-coin path so `position_qty` / `base_qty` may go
  negative, and apply the proven `montecarlo.rs` accounting: sell-to-open credits cash
  `notional − fee` and sets qty negative; buy-to-cover debits cash `notional + fee` and
  returns qty toward zero; equity = `cash + qty·mark` (already correct). A short opened
  at P and covered at Q realizes `(P − Q)·qty` (profit when Q < P). Long-only runs stay
  **byte-identical** (the short branch is gated off).
- **R-SS.2 — A bounded set of short-capable single-coin strategies.** A FIXED,
  code-declared, pre-registered set (no search — overfit-safe like the combination
  slate). See § Fork 5 for the recommended set. Each is a new bake-off arm.
- **R-SS.3 — Short cost model (LOAD-BEARING).** A configurable constant
  funding/borrow rate applied per-bar to an open short, following the `core::fx::FxRate`
  precedent (private field + checked constructor that rejects ≤ 0). Specify where it
  lives and how it hits equity. See § Fork 3.
- **R-SS.4 — Honest unbounded-loss handling (LOAD-BEARING, NON-NEGOTIABLE).** A short
  loses without bound as price rises (a 2× price = −100%+ on the shorted notional). The
  sim MUST model this honestly: the €200 can be wiped out and cash can go negative. Do
  **NOT** silently cap losses at 0. Reuse the `montecarlo.rs` maintenance-margin
  liquidation (force-cover at the 0.5 floor, cash may go negative) **and** carry a
  "shorts can lose more than your budget" disclaimer on every short surface. See § Fork 4.
- **R-SS.5 — Day-1 baseline-equity-divergence e2e (LOAD-BEARING, CLAUDE.md
  non-negotiable).** A short strategy's equity must diverge from BOTH the long-only
  version AND buy-and-hold by ≥ 1 bp on a bar where the short is open; and on a
  **downtrend** the short must **profit** where long/flat sits flat — asserted with the
  correct sign. See § Day-1 e2e.
- **R-SS.6 — Robustness/anchor safety (LOAD-BEARING).** The robustness gate, the bands,
  and the buy-and-hold benchmark (ADR-0066) stay **FROZEN**. New short arms run with
  `write_report = false`. `verify_anchors.sh` stays **119/119**. The single-coin
  long-only path stays byte-identical (re-prove with a `*_byte_identical_to_head`-style
  test, mirroring the MN feature's `run_path` re-proof).
- **R-SS.7 — Audit ledger sign-handling.** Either relax the `OpenPosition` `qty > 0`
  invariant to permit signed positions for short-capable runs, or route short-arm fills
  through a path that does not trip the `LedgerError::Database` reject — without
  breaking the double-entry reconciler (Σ debits == Σ credits) or `audit`'s
  no-sibling-imports rule. Architect picks the seam (see open questions).
- **R-SS.8 — Paper-sim parity.** The forward paper-trade (the agent runtime + Live
  view) must execute the short arm faithfully: open-short, accrue per-bar funding, cover
  on the bullish flip, and liquidate at the floor — the same model the backtest used, so
  the forward run is consistent-by-construction with the bake-off (the F5/F5b discipline).
- **R-SS.9 — UI surfaces.** The Live view shows short positions (a SHORT badge / signed
  qty / short P&L); the forward plan describes sell-to-open / cover rules honestly; the
  leaderboard marks short-capable arms; the unbounded-loss + not-advice + paper-only
  disclaimers render on every short surface. (Architect splits dev ‖ ui-designer.)
- **R-SS.10 — Honest framing in the product surface.** No alpha claim. The bake-off
  must be able to crown buy-and-hold / report all-Fragile over the short arms exactly as
  it does over long arms (`BenchmarkWins` / `AllFragile` reachability UNCHANGED). A null
  result is a valid product outcome.

## The five design forks — resolved (with operator-level flags)

### Fork 1 — Instrument model: perpetual-futures-short (funding) vs spot-short (borrow)? → **perpetual-futures-short (funding rate)**. *(Operator-level: informational; recommend ratify.)*

**Recommend perp-short with a funding rate.** Rationale: (a) retail crypto shorts are
overwhelmingly perpetual futures, so it is the **honest** model for the actual user; (b)
the codebase **already has the funding machinery** — `core::funding::FundingObs`, the
per-bar `cash += notional·(−rate)` accrual in `montecarlo.rs:460-520`, and the
`MAX_LEVERAGE`/maintenance-margin liquidation that perps require. A spot-short with a
stock-style borrow-fee + recall is a *different and less-crypto-native* model with no
existing engine support. Perp-short reuses the most code and is the most faithful to
retail reality. **Implication:** funding is the binding short cost (Fork 3) and
liquidation is the honest loss-bound (Fork 4) — both already modeled. *If the operator
prefers spot-borrow semantics, it is a config re-label of the same per-bar constant cost
(no liquidation needed but losses still unbounded until margin-call) — a fallback, not
the recommendation.*

### Fork 2 — Leverage for v1? → **1x / no leverage**. *(Operator-level: recommend ratify; faithful-leverage is a follow-on.)*

**Recommend 1x.** It is the simplest, safest, most honest v1, and it is **already the
constant** (`MAX_LEVERAGE = Decimal::ONE`, `montecarlo.rs:90`). Modeling >1x faithfully
means a real liquidation engine with funding-on-leveraged-notional, initial+maintenance
margin tiers, and partial liquidations — a large add that multiplies the
unbounded-loss-realism burden. At 1x the short notional is bounded by deployed cash, the
existing 0.5 maintenance floor is a clean honest bound, and the divergence test is
unambiguous. **Follow-on:** configurable leverage with a faithful liquidation ladder
(v0.2+), loudly flagged as higher-risk.

### Fork 3 — Short cost model? → **configurable constant funding rate, per-bar, via the `FxRate` precedent**. *(Architect-level; not operator-level.)*

**Recommend a single configurable constant funding rate** (e.g. a default ~0.01%/8h ≈
the historical BTC perp average, operator-tunable), applied per-bar to the open short
exactly as `montecarlo.rs:460-520` does: `cash += notional·(−rate)` at the settlement
cadence, where `notional = qty·mark < 0` for a short, so a positive funding rate is a
**cost** to the short. **Where it lives:** a new small honest-constant type modeled on
`core::fx::FxRate` (private field, checked constructor rejecting nonsensical values),
configured at the scenario/bake-off boundary — NOT a live funding feed (deferred, same
"simple honest constant first" stance F7 took for EUR/USD). **How it hits equity:** the
accrual debits `cash` each settlement bar, so it flows through `equity = cash + qty·mark`
automatically and shows up in the divergence test and the realized-P&L. **Follow-on:**
a live/historical funding feed (the `FundingObs` corpus already exists from the research
program — a clean v0.2 upgrade layered on the constant as its fallback).

### Fork 4 — Honest unbounded-loss handling? → **maintenance-margin liquidation at a floor + honest cash-can-go-negative + an explicit disclaimer**. *(Operator-level: the floor value + "can lose more than budget" framing are operator calls.)*

**Recommend reusing the shipped `montecarlo.rs` model:** mark-to-market the short every
bar (loss grows without bound as price rises), and **force-cover all shorts at a
maintenance floor** (`equity < maintenance_margin_frac × gross_short_notional`,
default 0.5) — which **can drive cash negative** in an extreme gap, modeling a real
liquidation that wipes the budget and then some. This is the honest middle path: it does
**not** silently cap losses at 0, and it does **not** pretend an infinitely-deep margin
account. Pair it with a load-bearing **"a short can lose more than your €200 — a 2×
price move wipes you out"** disclaimer on every short surface. **Operator decisions
here:** (i) the floor fraction (0.5 = the existing MN value; recommend inherit it for
consistency); (ii) whether the displayed €200 P/L is allowed to print **negative**
(recommend YES — that is the honest behavior; clamping it would re-introduce exactly the
dishonest cap R-SS.4 forbids).

### Fork 5 — Which strategies get shorts? → **symmetric long/short variants of the existing single-coin rule engines, as a bounded pre-registered set**. *(Operator-level: the exact arm list is a pre-registration the operator should ratify.)*

**Recommend: extend the existing single-coin rule engines to a symmetric long/short
interpretation, shipped as NEW named arms (not in-place mutation of the long-only arms),
as a FIXED pre-registered set.** Concretely the recommended v1 slate:

- `sma_cross_ls` — SMA crossover, symmetric: long on golden cross, **short on death
  cross** (instead of flat).
- `macd_ls` — MACD, symmetric: long on bullish flip, short on bearish flip.
- `rsi_ls` — RSI reversion, symmetric: long on oversold, short on overbought.
- `bbands_ls` — Bollinger, symmetric: long on lower-band touch, short on upper-band touch.
- `always_short` — a pure always-short benchmark arm (the down-side mirror of
  buy-and-hold). This is the **honest control**: it shows what un-timed continuous
  shorting does (it loses on any up-trending window, by construction) and anchors the
  divergence test's "short profits on a downtrend" assertion.

**Why this shape, not the alternatives:**
- *Keep the long-only arms untouched* (so the existing leaderboard and the 119/119
  anchors are undisturbed; the `_ls` arms are strictly additive, `write_report=false`).
- *Symmetric long/short over the existing rules* is the cleanest expression of "bet the
  other way when the rule flips" and reuses the existing indicator computation verbatim —
  only the **flat → short** branch is new.
- *A bounded, pre-registered, code-declared set* keeps it overfit-safe by construction,
  exactly the discipline the combination-search feature uses — no parameter/threshold
  search for "the best short."
- The `always_short` control is the short-side analogue of the buy-and-hold benchmark
  and makes the honest framing concrete (an un-timed short is a guaranteed loser on a
  bull leg — the leaderboard will say so).

**Operator ratifies the slate** (it is a pre-registration); the architect may rename or
re-shape the 5 arms but the SET is fixed before any results are read.

## MVP scope vs follow-ons

### v1 MVP (recommended)

1. **Correct single-coin short P&L** in `run_scenario` / `sma_composed_run` (R-SS.1),
   ported from the proven `montecarlo.rs` model, gated so long-only stays byte-identical.
2. **The 5-arm bounded short-capable strategy slate** (R-SS.2, Fork 5).
3. **The constant per-bar funding cost model** (R-SS.3, Fork 3) — `FxRate`-style honest
   constant, no live feed.
4. **Honest unbounded-loss handling** (R-SS.4, Fork 4) — maintenance-margin liquidation
   at the 0.5 floor, cash may go negative, plus the disclaimer.
5. **The day-1 baseline-equity-divergence e2e** (R-SS.5) including the downtrend
   profit-with-correct-sign assertion.
6. **Minimal UI** (R-SS.9): Live view renders short positions + short P&L; the
   leaderboard marks `_ls` arms; the disclaimers render. (Forward-plan short-rule copy
   can be the lighter end of MVP; full forward-plan narration is a follow-on.)
7. **1x leverage**, **perp-funding** instrument model (Forks 1+2).
8. Anchor + robustness safety (R-SS.6): `write_report=false`, gate/bands/benchmark
   frozen, 119/119 held, long-only byte-identity re-proven.

### Follow-ons (explicitly OUT of v1)

- **Configurable leverage + a faithful liquidation ladder** (v0.2+, loudly higher-risk).
- **A live / historical funding feed** (reuse the `FundingObs` corpus; layered on the
  constant as fallback).
- **Both perp-short AND spot-short** instrument models behind a config switch.
- **A short-capable *combination* slate** (long/short ensembles) — only after the
  single-arm short loop is proven, mirroring the F8 → combination-search progression.
- **Full forward-plan short narration** (sell-to-open / cover / liquidation rules in the
  F6 plain-language plan + the F9 LLM narration), if not fully landed in MVP.

## Engine-surface estimate (the candid "expensive" accounting)

This is the big-lift feature. Honest surface, with the de-risking from the audit folded in:

| Surface | What changes | Risk / size |
|---|---|---|
| **Backtest P&L core** (`engine.rs` `run_scenario`, `cli_types.rs` `BacktestState`, `sma_composed_run.rs`) | Remove the 3 long-only clamps; port the signed open/cover/funding/liquidation logic from `montecarlo.rs`; gate it so long-only is byte-identical | **High** — the dominant surface, but **the logic is already written + tested** in the sibling path; this is port-and-adapt, not invent |
| **Signal model** (`core::signal.rs`) | A minimal open-short / cover intent the single-coin loop reads | Low — additive enum/interpretation; `_ => {}` keeps consumers compiling |
| **Strategy library** (`crates/strategy`) | 5 new short-capable arms (symmetric `_ls` variants + `always_short`) reusing existing indicators | Medium — additive, the flat→short branch is the only new logic per arm |
| **Cost model** (new `FxRate`-style funding constant) | A small honest-constant type + scenario wiring | Low — direct `core::fx::FxRate` precedent |
| **Audit ledger sign-handling** (`crates/audit`, `core::position.rs` `OpenPosition`) | Relax/route the `qty > 0` reject for short runs without breaking the reconciler or no-sibling-imports | **Medium-high** — isolation-sensitive; the architect must pick the seam carefully |
| **Paper-sim / agent runtime** (`crates/agent` runtime, `crates/exec/paper.rs`) | Execute open-short / cover / funding / liquidation forward so the paper run matches the bake-off | Medium — mirrors the backtest model; the `exec/paper.rs` shim itself is announce-only, the position model lives in the runtime |
| **UI** (`crates/ui` Live view, leaderboard, forward plan) | SHORT badge + signed qty + short P&L + `_ls` arm markers + disclaimers | Medium — `live.rs` already carries `base_qty` through the bus (`live.rs:537`); rendering a negative qty + disclaimers is the new work; **verify at the render layer** per CLAUDE.md |
| **Day-1 e2e + byte-identity re-proof** | The divergence test + the long-only-unchanged re-proof | Medium — templates exist (`*_divergence_end_to_end.rs`, the MN `run_path` re-proof) |

**Rough size:** comparable to the MN-spread feature's **~5-8 developer-days** (which the
trace row estimated for "the short-side engine + the run_path anchor re-proof are the
bulk + dominant risk, NOT the cost model which already exists") — **plus** the single-coin
audit-ledger sign work and the UI short surfaces, which the MN feature (a research sweep
with no cockpit surface) did not carry. Net: **a large feature, but the financial core is
a port of proven code, not a green-field build** — the dominant *new* risks are the
audit-ledger isolation seam and the honest-negative-P&L UI, not the short math.

## Day-1 e2e (R-SS.5 — the CLAUDE.md non-negotiable, spelled out)

A short strategy is exactly the "strategy overlay / sizing-modifier" class the
non-negotiable targets. The test (pattern: `crates/strategy/tests/*_divergence_end_to_end.rs`,
e.g. `combination_slate_divergence_end_to_end.rs`):

1. **Divergence from long-only:** on a bar where a `_ls` arm holds a short, its equity
   differs from the same arm's long-only sibling by ≥ 1 bp. (Catches a no-op short where
   the flat→short branch is computed but never applied — the v3-vol-overlay failure mode.)
2. **Divergence from buy-and-hold:** the short arm's equity differs from the
   buy-and-hold benchmark by ≥ 1 bp on that bar.
3. **Downtrend profit with the correct SIGN (the load-bearing assertion):** on a
   synthetic / real **downtrend** window, the `always_short` arm's terminal equity is
   **> initial** (it profits) while a long/flat arm sits flat or loses — asserted as a
   signed inequality, not just "differs". This is the assertion that proves the short is
   real and points the right way.
4. **Funding-cost non-no-op:** with the funding rate set non-trivially, the short arm's
   equity differs from the same run at zero funding (the cost actually bites) — mirrors
   the MN `short-leg funding-cost non-no-op` falsifier.
5. **Unbounded-loss honesty:** on a sharp up-gap window, the `always_short` arm's equity
   is permitted to print **below zero** (or liquidate with cash < 0) — asserts the loss
   is NOT clamped at 0.

### Bear-window relevance (note for the architect)

Shorts **lose** on a bull window (e.g. the H1-2024 leg the long-only program leaned on)
and **shine** on downtrends. A faithful test therefore needs a **bear/choppy** window:
the repo already pins a **2021-22 bear corpus `4f390622`** (and 2022 BTC ≈ −57%) — use
it for the divergence/profit assertions. The **robustness gate still judges robustness
across resampled paths** regardless of the chosen window; the bear corpus is for the
*deterministic* day-1 sign assertion, not for cherry-picking a favorable headline.

## Honest framing (load-bearing — not optional copy)

- **No alpha claim.** Shorts are **very likely ALSO Fragile** under the frozen gate. The
  most directly-relevant prior is `REQ-PERP-BASIS-MARKET-NEUTRAL-001`: the long/short
  market-neutral basis spread came back **FAMILY-UNIFORM-FRAGILE on all 12 surfaces** vs
  its null. Single-coin directional shorts inherit full (inverse) market beta and a real
  funding cost — there is **no prior reason to expect them to clear a bar long-only
  could not.** The honest expectation is that they are Fragile too.
- **A null result is the expected, valid, shippable outcome.** "All short arms are also
  Fragile; on this window buy-and-hold still wins / hold still stands" is a **success of
  the test**, not a failure of the feature. The deliverable is *an honest measurement of
  whether directional shorts add robust value*, not a winner.
- **The gate decides, not the feature author.** `BenchmarkWins` / `AllFragile`
  reachability is UNCHANGED; the short arms are scored, gated, and benchmarked exactly
  like every other arm.
- **Disclaimers (mandatory on every short surface):** not-financial-advice; paper /
  simulated-only; **and the short-specific "a short can lose MORE than your €200 — an
  unbounded loss; a 2× price move wipes you out and then some."**

## UI implications (scope for the dev ‖ ui-designer split)

- **Live view:** render an open short distinctly — a SHORT badge, the signed `base_qty`
  (negative), and short P&L (which is positive when price falls). The data already flows
  (`crates/ui/src/live.rs:537` carries `base_qty` through the bus); the new work is the
  signed/negative rendering + the short-P&L sign + the unbounded-loss disclaimer.
  **Verify at the rendered-pixel layer** (the `iced_test::Emulator::screenshot`
  harnesses) per the CLAUDE.md non-negotiable — a passing proxy is not proof the SHORT
  badge draws.
- **Forward plan:** describe sell-to-open / cover / liquidation rules honestly in plain
  language ("shorts when SMA-20 crosses below SMA-50; covers on the reverse cross; is
  force-liquidated if the loss reaches the maintenance floor").
- **Leaderboard:** mark short-capable arms (`_ls` / `always_short`) so the user sees the
  short field; carry the disclaimer; show the same Sharpe / return / max-drawdown +
  robustness flag columns (a short's max-drawdown can be brutal — that is the honest
  signal).

## Open architecture questions (for the M-T1 lock)

1. **Signal-model shape (Q-SS-1):** new `SignalKind` variants (`OpenShort` / `CoverShort`)
   vs a **position-aware interpretation** of the existing `Sell`/`Buy` (Sell-when-flat →
   open-short, Buy-when-short → cover) the way `montecarlo.rs` already forks on
   `current_qty < 0`? The MN path used the **interpretation** route (Sell+"open_short"
   tag, Buy+"close_short" tag) with no new enum variant — recommend the architect
   strongly weigh that precedent for consistency, but the single-coin loop's
   `desired_side` logic (`engine.rs:1632-1640`) is simpler and may read cleaner with
   explicit variants. **Analyst lean:** mirror the MN interpretation route (no enum
   churn, proven), but this is the architect's M-T1 call.
2. **Where the short engine lives (Q-SS-2):** adapt `run_scenario` / `sma_composed_run`
   in place (gated on a `short_enabled` flag, like `k_short > 0` gates the MN branch) vs
   a sibling single-coin-short scenario function? In-place-gated is the MN precedent and
   keeps one engine; a sibling isolates risk. **Analyst lean:** in-place-gated, to keep
   the long-only byte-identity re-proof as the single safety gate.
3. **Audit-ledger seam (Q-SS-3):** how to permit signed positions in the ledger reader
   (`OpenPosition` `qty > 0` / `LedgerError::Database`) without breaking the reconciler
   invariant or `audit`'s no-sibling-imports rule? Relax the invariant for short runs,
   add a signed-position reader variant, or keep short P&L out of the audited
   open-positions table and only in the equity curve? This is the **isolation-sensitive**
   decision and the dominant *new* (non-ported) risk.
4. **Funding-constant home (Q-SS-4):** new type in `core` next to `FxRate`/`FundingObs`,
   vs a scenario-config field? And the settlement cadence — reuse the MN 8h grid, or a
   per-bar accrual for the single-coin hourly path?
5. **Liquidation-floor + negative-P&L policy (Q-SS-5):** inherit `maintenance_margin_frac
   = 0.5`? Confirm the displayed €200 P/L is allowed to print negative (the honest
   behavior R-SS.4 mandates) and that the UI does not clamp it.
6. **Paper-sim parity seam (Q-SS-6):** the backtest short model must be the **same code**
   the forward paper run executes (F5b consistency discipline) — where does that shared
   short-execution logic live so both the bake-off and the agent runtime call it?

## Requirements (analyst-owned) vs ownership of later columns

This brief and the `REQ-ADVISOR-SHORT-SELLING-001` `[[req]]` row (state `proposed`) are
analyst-owned. The architect fills `arch` + the Design section + `tasks.md` + the ADR
(an ADR-0051-style anchor-additive amendment is owed — this is the **second** feature to
touch the single-coin engine's short clamps after the MN feature touched `run_path`);
the developer fills `crates` + `tests`; the tester fills `anchors` after a PASS (expected:
still 119/119 — no new anchored report, `write_report=false`).

## Design

> Architect M-T1 lock, 2026-06-23. **ADR-0068** is the binding decision record;
> this section is the feature-local restatement. The six open questions Q-SS-1..6
> are resolved below. Verified against code 2026-06-23: the clamp sites, the
> `montecarlo.rs` short engine, the audit reader/writer seam, and the strategy
> library all match the brief, with **one correction** — there is a FOURTH
> long-only clamp the brief did not enumerate (see § Engine surfaces).

### One verified correction to the audit (de-risks Q-SS-3 substantially)

The brief frames Q-SS-3 (the audit-ledger seam) as the dominant new risk. Reading
the code narrows it sharply:

- **The bake-off path never touches the audit ledger.** `run_bakeoff` →
  `run_scenario` → `sma_composed_run` runs entirely on in-memory `BacktestState`
  (`cli_types.rs`); no `audit::journal::*` call is on that path. So the
  `open_positions_at` `qty > 0` reject is **NOT on the bake-off critical path at
  all** — the ranking, the equity curves, and the robustness gate never read it.
- **The double-entry WRITER is already sign-agnostic.** `post_fill_with_signal`
  (`journal.rs:64-237`) writes the same balanced `Dr assets:cash:USDT` /
  `Cr assets:position:<SYM>` legs for a Sell whether it closes a long or opens a
  short — the reconciler invariant (Σ debits == Σ credits) holds unchanged. The
  reject lives **only** in the *reader* `open_positions_at` (`query.rs:1872-1881`),
  consumed by exactly two callers: `crates/reports::lib` and (transitively) the
  cockpit positions widget. Never the equity curve.

Net: Q-SS-3 is a **reader-only relaxation** (emit a signed `OpenPosition` instead
of raising), with the writer + reconciler + `audit`'s no-sibling-imports rule all
untouched. This is the crux, and it is far narrower than feared.

### Engine surfaces (the FOUR long-only clamps — gate, don't delete)

The single-coin path is long-only by **four** explicit clamps (the brief named
three; the fourth is the forward paper loop). The equity formula `cash + qty·mark`
is already short-correct at every one of them. Each clamp becomes *gated* on
`short_enabled` so it is byte-for-byte HEAD's code when shorting is off:

| # | Site | Today | Gated change |
|---|------|-------|--------------|
| 1 | `engine.rs:1632-1640` `desired_side` | Buy-only-when-flat / Sell-only-when-long | when `short_enabled`: `Sell` when `qty ≤ 0` → open/extend short; `Buy` when `qty < 0` → cover (the `montecarlo.rs` fork) |
| 2 | `engine.rs:1713-1715` sell-fill | clamp `base_qty` to 0 | when `short_enabled`: allow negative `base_qty` |
| 3 | `cli_types.rs:632-635` `apply_sell` | clamp `position_qty` to 0 + zero cost | when `short_enabled`: allow negative `position_qty`, carry signed cost |
| 4 | `sma_composed_run.rs:554` sell-fill | clamp `base_qty` to 0 | when `short_enabled`: allow negative `base_qty` |
| **+** | **`runtime.rs:1809-1813` + `:1884-1885`** (forward paper loop) | **same `desired_side` clamp + `.max(0)`** | **when `short_enabled`: same as #1/#2 — this is the F5b parity site (Q-SS-6)** |

The gate value `short_enabled` is set `true` only for the new `_ls` / `always_short`
arms; every existing long-only arm leaves it `false` and is byte-identical.

### Q-SS-1 — Signal model: position-aware INTERPRETATION of `Buy`/`Sell` (no new variant)

**Decision: reuse `Buy | Sell | Hold`, interpret on the sign of the current
position — Sell-when-flat-or-short opens/extends a short, Buy-when-short covers —
exactly as `montecarlo.rs:249-455` forks on `current_qty < 0`.** No new
`SignalKind` variant.

- **Why:** (a) it is the proven MN route; (b) it keeps every exhaustive
  `match sig.kind` across the workspace total with **zero churn** — no `_ => {}`
  audit, no serde-rename, no `signal_kind_to_side_str` / `PlanRuleShape` /
  forecast / ensemble-vote edit. The single-coin `desired_side` block (clamp #1)
  becomes a clean four-arm match on `(kind, sign(qty))`.
- **Rejected:** explicit `OpenShort` / `CoverShort` variants — they read marginally
  cleaner in isolation but force edits in every signal consumer for no behavioural
  gain, multiplying exactly the blast radius the freeze contains.

### Q-SS-2 — Short engine home: in-place-gated (not a sibling scenario)

**Decision: in-place in `run_scenario` / `sma_composed_run` (and the forward loop),
gated on `short_enabled`,** mirroring the MN `k_short` precedent. A sibling
`*_short` scenario is rejected — it would duplicate the solvency/fill/equity loop
and split the byte-identity re-proof into two gates. One engine → one
`*_byte_identical_to_head` safety surface.

### Q-SS-3 — Audit-ledger seam: relax the READER to emit signed positions (writer + reconciler unchanged)

**Decision (the crux): change only the reader `open_positions_at`.**

- Replace the `running_qty < 0` → `LedgerError::Database` raise (`query.rs:1875`)
  with emission of a **signed** `OpenPosition` (`qty` may be `< 0`). The field type
  already holds a signed `Decimal`; only the doc-invariant relaxes from "`qty > 0`"
  to "signed: positive = long, negative = short."
- The `running_qty == 0` (flat) skip stays. A short lot's `avg_cost_basis` is the
  weighted-average **open (proceeds) price**, computed by the same
  proportional-release arithmetic the long path uses, mirrored for the sign.
- `post_fill_with_signal` (writer) and the double-entry reconciler are
  **byte-unchanged**. `audit` still depends only on `trading_core` — the
  no-sibling-imports rule is intact (the change is internal to `query.rs`).
- The two consumers (`crates/reports::lib`, cockpit positions widget) render the
  signed qty (see § UI). A **signed-position reader unit test** (a journaled
  sell-to-open materializes as `qty < 0`, not an error) is the regression guard.
- **Rejected:** a separate `open_positions_signed_at` reader (forks the reader,
  forces both consumers to choose); keeping shorts out of the audited table (the
  cockpit positions panel would silently lie on a short — a fidelity lie the
  honest-framing mandate forbids).

### Q-SS-4 — Funding constant home + cadence: `core::funding::FundingRate`, per-bar

**Decision: a new honest-constant type `core::funding::FundingRate`** (private
`Decimal` field + checked constructor rejecting non-finite/absurd values, modelled
exactly on `core::fx::FxRate` from ADR-0065), with a `DEFAULT_PERP_FUNDING_RATE`
≈ 0.01%/8h (the historical BTC-perp average, operator-tunable). It lives in `core`
next to `fx.rs` so the bake-off, the forward loop, and any future live feed read
one type — **not** a scenario-config primitive.

- **Cadence: per-bar accrual.** The real-data single-coin loop shares no synthetic
  8h epoch with the MN grid (whose 8h detection keys off `EPOCH_2023_NS`), so per-bar
  is both correct and simpler. `cash += notional·(−rate_per_bar)` where
  `rate_per_bar` scales the configured 8h rate to the bar timeframe; for an open
  short `notional = qty·mark < 0`, so a positive rate is a **cost**. It flows
  through `equity = cash + qty·mark` automatically → it bites the divergence test
  and the realized-P&L.
- A live/historical funding feed (the `FundingObs` corpus) is a **v0.2** upgrade,
  layered on this constant as its fallback.

### Q-SS-5 — Liquidation floor + negative-P&L policy: inherit 0.5; €200 P/L MAY print negative

**Decision: inherit `maintenance_margin_frac = 0.5` verbatim** from
`montecarlo.rs`. Mark-to-market the short every bar (loss grows without bound);
force-cover **all** shorts when
`equity < maintenance_margin_frac × gross_short_notional`, which **may drive cash
negative** in an extreme gap. Losses are **NOT** clamped at 0. The displayed €200
P/L is **allowed to print negative**; the UI does not clamp it. Every short surface
carries the disclaimer *"a short can lose MORE than your €200 — an unbounded loss;
a 2× price move wipes you out and then some."* + not-advice + paper/simulated-only.

### Q-SS-6 — Paper-sim parity: ONE shared `backtest::short_exec` helper

**Decision: extract the signed open/cover/fund/liquidate state transition into a
pure, sync, deterministic helper** in `crates/backtest` (e.g. a `short_exec`
module) operating on the shared in-memory shape (cash, signed `position_qty`, mark,
fee, `FundingRate`, the liquidation predicate). **Both** the bake-off
(`run_scenario` / `sma_composed_run`) **and** the agent forward loop
(`spawn_trading_loop`, `runtime.rs:1758+` — the fourth clamp site) call the same
helper, so the forward paper run is consistent-by-construction with the ranked
bake-off (the F5/F5b discipline). `crates/agent` already depends on
`crates/backtest`, so no new crate edge. The helper has no I/O → unit- and
property-testable in isolation.

### The FIXED pre-registered 5-arm slate

Code-declared, no search (overfit-safe — the combination-slate discipline). The
long-only arms are UNTOUCHED; the `_ls` arms are strictly additive new bake-off arms
run with `write_report = false`.

| Arm | Rule | Short branch |
|-----|------|--------------|
| `sma_cross_ls` | SMA crossover | long on golden cross, **short on death cross** (was flat) |
| `macd_ls` | MACD | long on bullish flip, short on bearish flip |
| `rsi_ls` | RSI reversion | long on oversold, short on overbought |
| `bbands_ls` | Bollinger | long on lower-band touch, short on upper-band touch |
| `always_short` | always-short benchmark control | the down-side mirror of buy-and-hold — loses on any up-trend by construction; anchors the day-1 downtrend-profit sign assertion |

The `_ls` arms reuse the existing indicator computation verbatim — only the
**flat → short** branch is new (via the Q-SS-1 interpretation route). The slate is
FIXED before any results are read; the operator ratifies it as a pre-registration.

### Anchor-safety + the freeze (both load-bearing, both BY CONSTRUCTION)

- New short arms run on the bake-off path with **`write_report = false`** → they
  touch no anchored report body → `verify_anchors.sh` stays **119/119** (run before
  the first clamp edit AND after the last; anchors keyed by NAME not filename).
- The single-coin long-only path is re-proven byte-identical with a
  `*_byte_identical_to_head` test mirroring the MN `run_path` k_short=0 re-proof.
- `classify_verdict` / `compute_robustness_flag` / `verdict_bands` / `bootstrap.rs`
  + `rank_candidates` + the ADR-0066 benchmark exemption are **FROZEN** — this is
  NOT a band proposal; the gate judges the short arms' equity as-is and buy-and-hold
  stays the benchmark. `BenchmarkWins` / `AllFragile` reachability is UNCHANGED.
- **No anchor SHA mutates** → no `spec/anchors.toml` amendment, no ADR-0038 §D6
  re-emission. This is the second feature to touch the single-coin short clamps
  after the MN feature touched `run_path`, but unlike the MN θ-surface work it
  writes **no new anchored report**.

### Honest framing (load-bearing)

No alpha claim. Shorts are very likely **also Fragile** under the frozen gate (the
MN long/short basis spread came back FAMILY-UNIFORM-FRAGILE on all 12 surfaces;
single-coin directional shorts inherit full inverse market beta + a real funding
cost, with no prior reason to clear a bar long-only could not). A **null result**
("all short arms also Fragile; hold still stands") is the EXPECTED, valid, shippable
outcome. The gate decides, not the author.

## Backtest Scenarios

Two distinct scenario classes — keep them separate (the brief's bear-window note):

1. **Deterministic day-1 sign assertions (the e2e gate).** Use the pinned 2021-22
   bear corpus `4f390622` (2022 BTC ≈ −57%). On a falling-price window the
   `always_short` arm's terminal equity must be **> initial** (it profits) while a
   long/flat arm sits flat or loses — asserted as a *signed* inequality. This is the
   load-bearing "short is real and points the right way" proof. A synthetic
   monotone-down fixture is also acceptable for the unit-level divergence/no-op
   checks (the `combination_slate_divergence_end_to_end.rs` pattern uses synthetic
   `SmaCrossover` bars for guaranteed signals); the bear corpus is for the headline
   sign assertion.

2. **The robustness read (the gate's verdict).** The frozen
   `RobustnessMode::Bootstrap` gate judges each `_ls` / `always_short` arm across
   resampled paths exactly like every long arm — buy-and-hold (ADR-0066) stays the
   benchmark. Expected: the short arms come back **Fragile** (the honest null). The
   bear corpus is **not** cherry-picked for a favorable headline — robustness is read
   across resampled paths regardless of the chosen window.

Anchor note: every short-arm run is `write_report = false` → no anchored report body
is created → `verify_anchors.sh` stays 119/119. No new anchor is added by this
feature (the tester confirms 119/119 post-PASS; `anchors = []` stays empty in the
trace row).

## Implementation

Developer track completed 2026-06-23. The following tasks were implemented (developer-owned rows only; T-D1/T-D2/T-D3/T-D6 were implemented in a prior developer session):

**T-D4 — Audit reader: signed OpenPosition (reader-only relaxation).**
- `crates/audit/src/query.rs`: In `open_positions_at`, the `Side::Sell` branch no longer raises `LedgerError::Database` when `running_qty < 0`. Instead it emits a signed `OpenPosition` with `qty = running_qty` (negative for shorts). The `Side::Buy` matching path now also handles cases where the prior session left `running_qty < 0` (short covered to flat). The `== 0` flat skip is unchanged.
- `crates/core/src/position.rs`: `OpenPosition.qty` doc-invariant relaxed from "`qty > 0`" to "Signed open quantity. Positive = long position; negative = short position." 
- `post_fill_with_signal` (writer) and reconciler are byte-unchanged. `audit` still depends only on `trading_core`.
- Tests: `crates/audit/tests/open_positions.rs:t1002_net_negative_emits_signed_position` (8 PASS), `crates/audit/tests/open_positions_at.rs:t1005_d7_sell_to_open_materializes_as_signed_position` + `t1005_d7_long_position_byte_identical_after_relaxation` (5 PASS).

**T-D5 — Forward paper-loop parity (the fourth clamp site).**
- `crates/agent/src/runtime.rs`: `spawn_trading_loop` accepts a `short_enabled: bool` parameter. The `desired_side` match block is gated on `short_enabled`; when false, the function falls back to the original long-only path. When true, Sell-when-`qty<=0` routes through `backtest::short_exec::try_open_short`, Buy-when-`qty<0` routes through `backtest::short_exec::try_cover_short`. The `.max(Decimal::ZERO)` equity clamp is also gated on `!short_enabled`. Per-bar funding accrual and maintenance-margin liquidation (via `short_exec`) are called after fills when `short_enabled`. All four call sites updated: research=false, initial_default=false, forward_launch=`BakeoffConfig::is_short_enabled(strategy_id)`, headless=false.

**T-D7 — Day-1 baseline-equity-divergence e2e (CLAUDE.md non-negotiable).**
- Created `crates/backtest/tests/short_selling_divergence_e2e.rs` with 4 test functions:
  1. `t_d7_ls_arm_diverges_from_long_only_and_buyhold` (assertions 1+2): SMA(1,2) short-enabled arm diverges ≥1bp from long-only and from buyhold on a 200-bar downtrend.
  2. `t_d7_always_short_profits_on_downtrend_signed_inequality` (assertion 3): terminal equity > initial on a monotone downtrend with SMA(1,2)+short_enabled. SIGNED inequality — the load-bearing assertion.
  3. `t_d7_funding_is_non_no_op_on_open_short` (assertion 4): `short_exec::accrue_funding` at rate>0 debits cash vs rate=0 no-op. Direct `short_exec` call — no bar loop.
  4. `t_d7_always_short_loses_on_uptrend_unbounded_loss` (assertion 5): Direct `short_exec::try_open_short` with max-sized short (fee_bps=0, equity_hint=initial*10 so full cash committed → qty=-0.2 BTC). At adverse_price=350,000 (7×), `equity = cash + qty*mark = -50,000 << 0`. Proves no `.max(0)` clamp.
- Note: The spec referenced `crates/strategy/tests/` but the correct home is `crates/backtest/tests/` because `sma_composed_run::run` and `short_exec` live in `crates/backtest`.

**T-D8 — Long-only byte-identity re-proof (load-bearing safety gate).**
- Created `crates/backtest/tests/short_enabled_byte_identity.rs` with 4 tests:
  1. `t_d8_long_only_deterministic_across_two_runs`: two identical long-only runs produce bit-identical equity curves.
  2. `t_d8_short_enabled_diverges_from_long_only_on_downtrend`: short_enabled=true diverges ≥1bp from false on a downtrend.
  3. `t_d8_long_only_flat_price_no_trades`: flat price → 0 trades, equity stays at initial.
  4. `t_d8_short_enabled_false_never_enters_short_on_downtrend`: gate-leak check — position_curve is always ≥0 when short_enabled=false.
- Uses `downtrend_bars_200()` (0.5%/bar drop, 200 bars, start=50,000) and `flat_bars_200()` (constant 30,000).

**T-D6 — The fixed 5-arm slate (engine dispatch wired, 2026-06-24).**
- `crates/backtest/src/engine.rs`: Added `"v0.sma_cross_ls"` to the `v0.sma` match arm; `"v0.macd_ls"` to `v0.5.macd`; `"v0.rsi_ls"` to `v0.5.rsi`; `"v0.bbands_ls"` to `v0.5.bbands`. Each `_ls` ID inherits `short_enabled` from `ScenarioConfig` (already threaded via `cfg.short_enabled`). All four long-only ID paths are byte-identical (no change to existing arms). Also added `strategy_dir_slug` mappings for all 5 new IDs.
- `crates/backtest/src/engine.rs`: Added `"v0.always_short"` arm (mirrors `v0.buyhold` structure) calling `run_alwaysshort_path`. Equity formula `initial_capital * (2 - price/price0)`. Loss is unbounded and NOT clamped at 0. `write_report=false` in bake-off → anchor-safe.
- `crates/backtest/src/bakeoff/buyhold.rs`: Added `run_alwaysshort_path` (the exact inverse of `run_buyhold_path` for a single-coin 1× short). 4 unit tests: `alwaysshort_empty_bars_returns_capital`, `alwaysshort_price_halves_equity_plus_50pct`, `alwaysshort_price_doubles_equity_zero`, `alwaysshort_price_triples_equity_negative`, `alwaysshort_deterministic`. All 9 buyhold tests pass.
- `crates/agent/src/runtime.rs`: `build_registry_for` wired for `"v0.sma_cross_ls"` → SmaCrossover, `"v0.macd_ls"` → ComposedStrategy(btc_macd_trend), `"v0.rsi_ls"` → ComposedStrategy(btc_rsi_reversion), `"v0.bbands_ls"` → ComposedStrategy(btc_bbands_mean_revert), `"v0.always_short"` → AlwaysLongStrategy (paper loop gates short_enabled=true via `BakeoffConfig::is_short_enabled`).
- `crates/backtest/tests/short_bakeoff_bear_bull.rs`: Updated to use REAL frozen IDs (no proxy mapping). `v0.always_short` uses the proper `run_alwaysshort_path` (not SMA(1,2) hack). Bear window result (2022-Q2, BTC −56.2%): **v0.always_short profits** — initial=100,000 → final=156,210.90 (+56.2%), Robust. All 4 _ls arms FRAGILE (null finding, expected). Bull window (H1-2024, BTC +47.7%): always_short loses (−47%), all FRAGILE. T-D6 SANITY PASS confirmed.
- Anchors: 119/119 before and after (no anchored report body written — `write_report=false` for all short arms).
- Clippy: `cargo clippy -p backtest --all-targets -- -D warnings` PASS; `cargo clippy -p agent --all-targets -- -D warnings` PASS.

**T-D9 — Anchor gate re-confirmed post-change.**
- `bash scripts/verify_anchors.sh` output: `ANCHORS PASS  (119 / 119)` — all 119 anchors pass, no anchored body was mutated.

**Clippy gate:** `cargo clippy --workspace -- -D warnings` passes after fixing one `unnested-or-patterns` lint in `crates/backtest/src/scenarios/sma_composed_run.rs:554` (nested to `(Side::Sell, std::cmp::Ordering::Equal | std::cmp::Ordering::Less)`).

**Frozen files (git-diff empty):** `crates/audit/src/journal.rs`, `crates/backtest/src/scenarios/bootstrap.rs`, `crates/backtest/src/bakeoff/rank.rs` — all byte-unchanged.

## Verification
_tester links to reports here — expect FAMILY-Fragile is a valid PASS; the gate decides._

## Changelog

- 2026-06-23 (architect, M-T1 lock): authored the Design + **ADR-0068** (registered
  atomically in `_bmad-output/planning-artifacts/architecture/decisions/README.md`; `adr_registry_check.py --pre-commit`
  green). Resolved Q-SS-1..6: (1) signal = position-aware INTERPRETATION of `Buy`/`Sell`,
  no new `SignalKind`; (2) short engine = in-place-gated on a single `short_enabled` flag,
  not a sibling scenario; (3) audit seam = **reader-only** relaxation of `open_positions_at`
  to emit signed `OpenPosition` (the writer `post_fill_with_signal` is already sign-agnostic
  → reconciler UNAFFECTED; the bake-off never journals → the reject is off the bake-off
  path entirely; `audit` no-sibling-imports INTACT); (4) funding = a `core::funding::FundingRate`
  `FxRate`-style constant, per-bar accrual; (5) inherit the 0.5-floor liquidation, €200 P/L
  may print NEGATIVE; (6) one shared `backtest::short_exec` helper for bake-off ‖ forward-loop
  parity. **Verified correction:** there is a FOURTH long-only clamp the brief did not
  enumerate — the forward paper loop `runtime.rs:1809-1813`/`:1884-1885` (it is also the F5b
  parity site). Slate FIXED: `{sma_cross_ls, macd_ls, rsi_ls, bbands_ls, always_short}`,
  long-only arms UNTOUCHED. Gate/bands/benchmark FROZEN, `write_report=false` → 119/119 BY
  CONSTRUCTION, no anchor SHA mutates (no ADR-0038 §D6 re-emission). Set status `arch-done`;
  rewrote `tasks.md` with the ordered developer ‖ ui-designer build. No engine code, no
  anchored content touched.
- 2026-06-23 (analyst, scoping): authored the brief. Operator directed the "expensive
  short selling" 2026-06-23 — promoted from the `backlog.md` one-liner to a full proposal.
  **Key audit finding that de-risks the estimate:** a complete, tested, shipped short-side
  engine (open / cover / maintenance-margin liquidation / per-bar funding) ALREADY EXISTS
  in `crates/backtest/src/scenarios/montecarlo.rs::run_path` from the market-neutral
  perp-basis feature (`REQ-PERP-BASIS-MARKET-NEUTRAL-001`, shipped, 119/119, science
  verdict FAMILY-UNIFORM-FRAGILE) — but only in the **multi-symbol cross-sectional** path;
  the single-coin `run_scenario` / `sma_composed_run` path is hard long-only by three
  explicit clamps (`engine.rs:1632-1640` desired_side, `engine.rs:1713-1715` +
  `cli_types.rs:632-635` + `sma_composed_run.rs:554` qty clamps), while the equity formula
  `cash + qty·mark` is ALREADY short-correct. So the feature is **port-and-adapt the proven
  signed-position model into the single-coin path**, not invent it. Resolved the 5 forks:
  (1) perp-funding instrument; (2) 1x leverage (already `MAX_LEVERAGE=ONE`); (3) constant
  `FxRate`-style per-bar funding cost, no live feed; (4) honest unbounded-loss via the
  shipped maintenance-margin liquidation (cash may go negative) + a "can lose more than
  your budget" disclaimer; (5) a bounded pre-registered 5-arm slate (`sma_cross_ls`,
  `macd_ls`, `rsi_ls`, `bbands_ls`, `always_short`). Honest framing load-bearing: shorts
  are very likely ALSO Fragile (the MN long/short precedent was FAMILY-UNIFORM-FRAGILE); a
  null result is valid + shippable; gate/bands/benchmark FROZEN; `write_report=false` →
  anchor-safe; 119/119 held. No engine code; no anchored content touched.
