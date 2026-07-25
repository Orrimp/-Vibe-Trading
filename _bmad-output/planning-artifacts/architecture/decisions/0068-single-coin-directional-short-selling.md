---
adr: 0068
title: Single-coin directional short-selling — port the montecarlo signed-position model, gate it, isolate the audit reader
status: accepted
date: 2026-06-23
supersedes: none
superseded-by: none
---

# ADR-0068: Single-coin directional short-selling (paper/sim)

## Context

Operator directive 2026-06-23 ("do the expensive short selling") promotes
`REQ-ADVISOR-SHORT-SELLING-001` from a backlog one-liner to a full feature: give the
single-coin paper advisor the **down-half lever** it never had. Today every single-coin
arm is structurally long-or-flat — a bearish signal parks the €200 in cash and the best
the advisor can say on a downtrend is "hold cash."

Three facts (verified in code, 2026-06-23) shape this decision:

1. **The single-coin path is long-only by FOUR explicit clamps, not by the math.** The
   equity formula `cash + qty·mark` (`cli_types.rs:592-594`, `runtime.rs:1802`) is
   **already short-correct** — a sell-to-open at P drives `qty` negative, so equity falls
   as mark rises. The clamps that forbid it: (a) `engine.rs:1632-1640` `desired_side`
   (Buy-only-when-flat / Sell-only-when-long); (b) `engine.rs:1713-1715` sell-fill
   clamp-to-zero; (c) `cli_types.rs:632-635` `apply_sell` clamp-to-zero (also zeroes
   cost); (d) `sma_composed_run.rs:554` duplicate clamp; **and a FOURTH the brief did not
   enumerate** — the forward paper loop `runtime.rs:1809-1813` (`desired_side`) +
   `:1884-1885` (`.max(Decimal::ZERO)`). All four must become *gated*.

2. **A complete, tested, shipped short-side engine already exists** in
   `crates/backtest/src/scenarios/montecarlo.rs::run_path` from the market-neutral
   perp-basis feature (`REQ-PERP-BASIS-MARKET-NEUTRAL-001`, shipped, 119/119, science
   verdict FAMILY-UNIFORM-FRAGILE): open-short with an initial-margin gate +
   `MAX_LEVERAGE = Decimal::ONE` (`:90`); buy-to-cover (`:253-299`); maintenance-margin
   liquidation (`:531-589`, `maintenance_margin_frac = 0.5`, cash may go negative);
   per-bar funding accrual `cash += notional·(−rate)` (`:460-520`). The whole branch is
   **gated on `k_short > 0`** → inert + byte-identical for long-only, proven by
   `run_path_k_short_zero_byte_identical_to_head`. So the feature is **port-and-adapt the
   proven model into the single-coin path, not invent it.**

3. **The audit ledger rejects shorts — but only in the READER, and only on the
   paper-sim path.** `audit::query::open_positions_at` raises `LedgerError::Database` on a
   net-negative running_qty (`query.rs:1872-1881`, "Long-only at v1+"). Critically: the
   **bake-off path never journals to the ledger** (it is pure in-memory `BacktestState`),
   and the double-entry *writer* `post_fill_with_signal` (`journal.rs:64-237`) is
   **already sign-agnostic** — a sell-to-open writes the same balanced Dr-cash / Cr-position
   legs whether it opens or closes, so the reconciler (Σ debits == Σ credits) is unaffected.
   The reject is a *reader* invariant consumed only by `crates/reports` and the cockpit
   positions widget — never by the equity curve. This is the dominant *new* (non-ported)
   risk, but it is far narrower than the brief feared.

Paper/sim only. NO live trading, NO real orders, NO real margin — the €200 is simulated.

## Decision

### D1 — Port the montecarlo signed-position model into the single-coin path; gate it on `short_enabled`.

Lift the proven open-short / buy-to-cover / maintenance-margin-liquidation / per-bar-funding
logic from `montecarlo.rs::run_path` into the single-coin engine path. The gate is a single
**`short_enabled: bool`** flag (the single-coin analogue of `k_short > 0`), threaded
additively through `ScenarioConfig` and `SmaComposedRunInput` as
`#[serde(default)]` (default `false`). Every new short branch is dead code when
`short_enabled == false`, so the long-only path is byte-for-byte HEAD's code. `MAX_LEVERAGE`
(1x) and `maintenance_margin_frac` (0.5) are **inherited verbatim** from `montecarlo.rs` —
v1 is 1x fully-collateralized.

### D2 — In-place-gated, not a sibling scenario (Q-SS-2).

The short capability lives **in-place** in `run_scenario` / `sma_composed_run` (and the
forward loop, see D6), gated on `short_enabled`, mirroring the MN `k_short` precedent. A
sibling `*_short` scenario is REJECTED: it would duplicate the solvency/fill/equity loop
and split the byte-identity re-proof into two gates. One engine, one gate
(`*_byte_identical_to_head`), is the single safety surface.

### D3 — Signal model: position-aware INTERPRETATION of `Buy`/`Sell`, no new `SignalKind` variant (Q-SS-1).

Reuse the existing `Buy | Sell | Hold` enum with the `montecarlo.rs` interpretation fork on
`current_qty`: **Sell-when-flat-or-short → open/extend short; Buy-when-short → cover.** No
new enum variant. Rationale: (a) it is the proven MN route; (b) it keeps every exhaustive
`match sig.kind` across the workspace total with ZERO churn (no `_ => {}` audit, no
serde-rename migration, no `PlanRuleShape` / `signal_kind_to_side_str` extension); (c) the
single-coin `desired_side` block becomes a four-arm `match` on `(kind, sign(qty))` that
reads cleanly. The `OpenShort`/`CoverShort` explicit-variant alternative is REJECTED — it
would force edits in every signal consumer (UI plan, audit `strategy_signals`, forecast,
ensemble vote) for no behavioural gain, multiplying the blast radius the freeze is meant to
contain.

### D4 — Funding cost: a `core::fx::FxRate`-style constant, per-bar accrual (Q-SS-4).

A new small honest-constant type **`core::funding::FundingRate`** (private `Decimal` field +
checked constructor rejecting non-finite / absurd values, modelled exactly on
`core::fx::FxRate` from ADR-0065), with a `DEFAULT_PERP_FUNDING_RATE` ≈ 0.01%/8h (the
historical BTC-perp average, operator-tunable). It lives in `core` next to `fx.rs`/`funding`
— NOT a scenario-config primitive — so the bake-off, the forward loop, and any future live
feed read one type. **Cadence: per-bar accrual** on the single-coin path (the hourly
single-coin loop has no 8h synthetic grid; the MN 8h-grid detection keys off a synthetic
epoch the real-data single-coin path does not share). Per-bar accrual is
`cash += notional·(−rate_per_bar)` where `rate_per_bar` is the configured 8h rate scaled to
the bar timeframe; for an open short `notional = qty·mark < 0`, so a positive rate is a
**cost**. It flows through `equity = cash + qty·mark` automatically → it bites the
divergence test and the realized-P&L. A live/historical funding feed (the `FundingObs`
corpus) is a v0.2 upgrade layered on this constant as its fallback.

### D5 — Honest unbounded-loss: maintenance-margin liquidation at the 0.5 floor; cash + €200 P/L may print NEGATIVE (Q-SS-5).

Inherit the `montecarlo.rs` model verbatim: mark-to-market the short every bar (loss grows
without bound as price rises); force-cover **all** shorts when
`equity < maintenance_margin_frac × gross_short_notional` (floor `0.5`, inherited), which
**may drive cash negative** in an extreme gap. Losses are **NOT** clamped at 0 — that is the
honest behavior R-SS.4 mandates. The displayed €200 P/L is **allowed to print negative**;
the UI does not clamp it. Every short surface carries the load-bearing disclaimer:
*"a short can lose MORE than your €200 — an unbounded loss; a 2× price move wipes you out
and then some."* plus not-financial-advice + paper/simulated-only.

### D6 — Paper-sim parity: ONE shared short-execution helper both the bake-off and the forward loop call (Q-SS-6).

Extract the signed open/cover/fund/liquidate state transition into a **pure, sync,
deterministic helper** in `crates/backtest` (e.g. `backtest::short_exec`) operating on the
shared in-memory shape (cash, signed `position_qty`, mark, fee, funding rate, the
liquidation predicate). Both `run_scenario`/`sma_composed_run` AND the agent forward loop
`spawn_trading_loop` (`runtime.rs:1758+`) call **the same helper** — so the forward paper
run is consistent-by-construction with the ranked bake-off (the F5/F5b discipline; the
forward loop is the FOURTH clamp site this feature gates). `crates/agent` already depends on
`crates/backtest`, so this adds no new crate edge. The helper has no I/O — it is unit- and
property-testable in isolation.

### D7 — Audit reader: relax `open_positions_at` to emit SIGNED positions; writer + reconciler UNCHANGED (Q-SS-3 — the crux).

The `post_fill_with_signal` writer is already sign-agnostic and stays **byte-unchanged**; the
double-entry reconciler is unaffected (a sell-to-open is a balanced Dr-cash / Cr-position
transaction exactly like a sell-to-close). The **only** change is in the *reader*
`open_positions_at` (`query.rs`): replace the net-negative `LedgerError::Database` raise with
emission of a **signed** `OpenPosition` (`qty` may be `< 0`). Concretely:

- `OpenPosition.qty`'s doc-invariant changes from "`qty > 0`" to "signed: positive = long,
  negative = short" (the field type already holds it; only the contract relaxes).
- The end-of-scan materializer drops the `running_qty < 0` raise and emits the signed row;
  the `running_qty == 0` skip (flat) stays.
- `avg_cost_basis` for a short lot is the weighted-average **open price** of the short (the
  proceeds basis), computed by the same proportional-release arithmetic the long path uses,
  mirrored for the sign.

This keeps `audit`'s **no-sibling-imports rule intact** (the change is internal to
`query.rs`; `audit` still depends only on `trading_core`). The `crates/reports` orchestrator
and the cockpit positions widget — the only two consumers — render the signed qty (D8/UI).
A signed-position **reader unit test** (a journaled sell-to-open materializes as
`qty < 0`, not an error) is the regression guard. **Rejected alternatives:** (a) a separate
`open_positions_signed_at` reader variant — REJECTED, it forks the reader and forces both
consumers to choose, doubling the surface; (b) keeping shorts out of the audited table and
only in the equity curve — REJECTED, it makes the cockpit positions panel silently wrong on
a short (a fidelity lie the honest-framing mandate forbids).

### D8 — UI: signed/short rendering, verified at the render-pixel layer; disclaimers mandatory.

Live view renders an open short distinctly (a SHORT badge + the signed/negative `base_qty` +
short P&L, which is positive when price falls); the leaderboard marks the `_ls` /
`always_short` arms; the forward plan describes sell-to-open / cover / liquidation rules in
plain language. The unbounded-loss + not-advice + paper-only disclaimers render on **every**
short surface. Per CLAUDE.md, all of this is verified at the **rendered-pixel layer**
(`iced_test::Emulator::screenshot` harnesses) with a negative control — a passing proxy is
not proof the SHORT badge draws. `crates/ui` imports no new crate (the data already flows:
`live.rs:537` carries `base_qty`; a negative value is the new render work).

### D9 — The FIXED pre-registered 5-arm slate; long-only arms UNTOUCHED.

A bounded, code-declared, pre-registered set (no search → overfit-safe, the
combination-slate discipline): `sma_cross_ls`, `macd_ls`, `rsi_ls`, `bbands_ls` (symmetric
long/short variants of the existing rule engines — long on the bullish flip, **short instead
of flat** on the bearish flip, reusing the existing indicator computation verbatim) +
`always_short` (the down-side mirror of buy-and-hold — the honest control that loses on any
up-trend by construction and anchors the divergence test's "short profits on a downtrend"
assertion). The existing long-only arms are **untouched**; the `_ls` arms are strictly
additive new bake-off arms. The slate is FIXED before any results are read; the operator
ratifies it as a pre-registration.

### D10 — Anchor-safety + the freeze (both load-bearing, both BY CONSTRUCTION).

New short arms run on the bake-off path with **`write_report = false`** → they touch no
anchored report body → `verify_anchors.sh` stays **119/119** by construction (run before the
first clamp edit AND after the last; anchors keyed by NAME not filename). The single-coin
long-only path is re-proven byte-identical with a `*_byte_identical_to_head` test mirroring
the MN `run_path` k_short=0 re-proof. The robustness gate (`classify_verdict` /
`compute_robustness_flag` / `verdict_bands` / `bootstrap.rs`), `rank_candidates`, and the
ADR-0066 buy-and-hold benchmark exemption are **FROZEN** — this is NOT a band proposal; the
gate judges the short arms' equity curves as-is and buy-and-hold stays the benchmark.
`BenchmarkWins` / `AllFragile` reachability is UNCHANGED. **No anchor SHA mutates**, so no
amendment to `spec/anchors.toml` and no ADR-0038 §D6 re-emission is triggered — this ADR is
the *second feature to touch the single-coin short clamps* after the MN feature touched
`run_path`, but unlike the MN θ-surface work it writes **no new anchored report**.

### D11 — Honest framing (load-bearing, not optional copy).

No alpha claim. Shorts are very likely **also Fragile** under the frozen gate — the most
directly-relevant prior is the MN long/short basis spread coming back FAMILY-UNIFORM-FRAGILE
on all 12 surfaces; single-coin directional shorts inherit full (inverse) market beta + a
real funding cost, with no prior reason to clear a bar long-only could not. A **null result**
("all short arms also Fragile; hold still stands / buy-and-hold still wins") is the EXPECTED,
valid, shippable outcome. The gate decides, not the author.

## Alternatives considered

- **A sibling single-coin-short scenario** (vs in-place-gated, D2) — REJECTED: duplicates the
  solvency/fill/equity loop and splits the byte-identity re-proof into two gates.
- **New `OpenShort`/`CoverShort` `SignalKind` variants** (vs interpretation, D3) — REJECTED:
  forces edits in every exhaustive signal consumer for no behavioural gain; multiplies the
  blast radius the freeze contains.
- **8h funding settlement grid on the single-coin path** (vs per-bar, D4) — REJECTED: the
  real-data single-coin loop shares no synthetic epoch with the MN 8h grid; per-bar accrual
  is exact for the hourly path and simpler.
- **Clamping the displayed €200 P/L at 0** (vs honest-negative, D5) — REJECTED: re-introduces
  exactly the dishonest unbounded-loss cap R-SS.4 forbids.
- **A separate `open_positions_signed_at` reader / keeping shorts out of the audited table**
  (vs relaxing the existing reader, D7) — REJECTED: forks the reader / silently lies on the
  cockpit positions panel.
- **>1x leverage with a faithful liquidation ladder** — DEFERRED to a v0.2 follow-on (loudly
  higher-risk); v1 is 1x, already the `MAX_LEVERAGE` constant.
- **A live/historical funding feed** — DEFERRED to v0.2, layered on the D4 constant as
  fallback (the `FundingObs` corpus already exists).
- **A short-capable combination slate** — DEFERRED to a follow-on after the single-arm short
  loop is proven (mirroring F8 → combination-search).

## Consequences

- **If the long-only byte-identity gate fails** (`verify_anchors.sh` ≠ 119/119, OR the
  `*_byte_identical_to_head` test diverges), the port leaked into the long-only path — a
  REGRESSION; ship is blocked until the gate is green. This is the #1 risk after D7.
- **If the day-1 downtrend e2e fails** (the `always_short` arm does not profit with the
  correct SIGN on the 2021-22 bear corpus `4f390622`, OR a `_ls` arm's equity does not
  diverge ≥1bp from its long-only sibling AND from buy-and-hold), the short branch is a no-op
  (the v3-vol-overlay failure mode) — ship blocked. This is the CLAUDE.md non-negotiable.
- **If the audit reader change leaks to the writer or the reconciler** (Σ debits ≠ Σ credits
  on a sell-to-open, OR `post_fill_with_signal` is edited), the double-entry invariant breaks
  — the reconciler test catches it. The seam is reader-only by design.
- **If the funding accrual is a no-op** (the short arm's equity equals the zero-funding run),
  the cost is not biting — caught by the funding-non-no-op falsifier (mirrors the MN
  `short-leg funding-cost non-no-op`).
- Mechanically enforced by: `scripts/verify_anchors.sh` (119/119, run twice), the
  `*_byte_identical_to_head` re-proof, the day-1 `short_directional_divergence_end_to_end.rs`
  e2e (divergence + signed-downtrend-profit + funding-non-no-op + unbounded-loss-honesty),
  the signed-position reader unit test, and the render-pixel UI snapshots with a negative
  control. The freeze is enforced by the byte-frozen `classify_verdict`/`rank_candidates`
  (ADR-0066 / ADR-0063 §D4) being untouched.

## Changelog
- 2026-06-23 (architect): initial accept. Resolves Q-SS-1..6 for
  `REQ-ADVISOR-SHORT-SELLING-001`. Port-from-`montecarlo.rs` (D1), in-place-gated (D2),
  `Buy`/`Sell` interpretation route — no new `SignalKind` (D3), `FxRate`-style per-bar
  `FundingRate` constant in `core` (D4), honest maintenance-margin liquidation with
  negative-allowed P/L (D5), one shared `short_exec` helper for bake-off ‖ forward-loop
  parity (D6), reader-only `open_positions_at` signed relaxation with the writer + reconciler
  untouched (D7 — the crux), render-pixel-verified UI + disclaimers (D8), the FIXED 5-arm
  slate `{sma_cross_ls, macd_ls, rsi_ls, bbands_ls, always_short}` (D9), 119/119 +
  long-only byte-identity + frozen gate/bands/benchmark BY CONSTRUCTION (D10), honest
  null-result-is-shippable framing (D11). Noted a FOURTH long-only clamp the brief did not
  enumerate: the forward paper loop `runtime.rs:1809-1813`/`:1884-1885`. No anchor SHA
  mutates (no `spec/anchors.toml` amendment, no ADR-0038 §D6 re-emission). Leans on
  ADR-0051 §D6.10 (the MN short-side engine), ADR-0065 (the `FxRate` constant precedent),
  ADR-0066 (the frozen benchmark exemption), ADR-0063 §D4 (the frozen classifier),
  ADR-0024 (audit raw-sqlx, no-sibling-imports), ADR-0016 (`OpenPosition` shape), ADR-0059
  (the bake-off `write_report=false` anchor-safety).
