---
slug: advisor-short-selling
status: proposed
owner: architect
updated: 2026-06-23
---

# Tasks — advisor-short-selling

> **Analyst-seeded stub — awaiting the architect M-T1 lock.** The ordered build
> below is a PLACEHOLDER. The architect owns this list: it must resolve the six
> open questions (`Q-SS-1..6`) in [`feature.md` § Open architecture questions](feature.md),
> author the Design section + the owed ADR (incl. the ADR-0051-style anchor-additive
> amendment — the SECOND feature to touch the single-coin engine's long-only clamps
> after the MN feature touched `run_path`), then replace this list with the real
> ordered tasks. Trace `REQ-ADVISOR-SHORT-SELLING-001`.

## Load-bearing constraints (carry into EVERY task — non-negotiable)

- **PAPER / SIM ONLY.** The €200 is simulated; shorts are simulated short positions.
  **NO live trading, NO real orders, NO real margin** (standing operator constraint).
- **Port-and-adapt, do NOT invent.** The proven short-side model (open / cover /
  maintenance-margin liquidation with honest cash-can-go-negative / per-bar funding)
  already exists, tested + shipped, in `crates/backtest/src/scenarios/montecarlo.rs::run_path`
  (the MN feature `REQ-PERP-BASIS-MARKET-NEUTRAL-001`). Reuse it. The single-coin
  equity formula `cash + qty·mark` (`cli_types.rs:592-594`) is **already short-correct** —
  the bug is the three long-only clamps (`engine.rs:1632-1640`/`:1713-1715`,
  `cli_types.rs:632-635`, `sma_composed_run.rs:554`), not the mark-to-market.
- **Gate / bands / benchmark FROZEN.** Do NOT touch `classify_verdict` /
  `compute_robustness_flag` / `verdict_bands` (ADR-0066 / ADR-0063 §D4). This is NOT a
  band proposal. Frame as "more arms face the same bar," never "we moved the bar."
- **Anchor-safe by construction.** New short arms run `write_report=false` on the
  bake-off path → touch no anchored body. `verify_anchors.sh` stays **119/119** — run it
  **before the first engine-clamp edit AND after the last** (anchors keyed by NAME, not
  filename). Re-prove the single-coin long-only path byte-identical with a
  `*_byte_identical_to_head` test (mirror the MN `run_path` k_short=0 re-proof).
- **Honest unbounded-loss — do NOT cap losses at 0.** Maintenance-margin liquidation at
  the floor (default 0.5, inherit the MN value); cash + the displayed €200 P/L are
  ALLOWED to print negative. Disclaimers ("a short can lose more than your €200" +
  not-advice + paper-only) on every short surface.
- **Day-1 baseline-equity-divergence e2e is the CLAUDE.md non-negotiable (R-SS.5).** It
  ships from day 1, including the downtrend "short PROFITS where long/flat sits flat"
  assertion **with the correct sign** on the 2021-22 bear corpus `4f390622`.
- **No alpha promise.** Shorts are very likely ALSO Fragile (the MN long/short precedent
  was FAMILY-UNIFORM-FRAGILE). A null result is valid + shippable. The gate decides.

## Placeholder ordered build (architect to replace at M-T1)

- [ ] T0 — Resolve `Q-SS-1..6`; author Design + the ADR + the anchor-additive amendment.
- [ ] T1 — Signal-model: the minimal open-short / cover intent (architect picks the
  shape — analyst leans the `montecarlo.rs` interpretation route: Sell-when-flat →
  open-short, Buy-when-short → cover, no new enum variant).
- [ ] T2 — Port the signed short P&L into the single-coin engine path, gated so the
  long-only path is byte-identical (`run_scenario` / `sma_composed_run`).
- [ ] T3 — The `FxRate`-style constant per-bar funding cost type + scenario wiring.
- [ ] T4 — Honest unbounded-loss: maintenance-margin liquidation, cash may go negative.
- [ ] T5 — The bounded pre-registered 5-arm short slate (`sma_cross_ls` / `macd_ls` /
  `rsi_ls` / `bbands_ls` + `always_short`).
- [ ] T6 — Audit-ledger sign-handling (the dominant new, isolation-sensitive risk —
  `OpenPosition` `qty>0` / `LedgerError::Database`), without breaking the reconciler or
  `audit`'s no-sibling-imports rule.
- [ ] T7 — Paper-sim parity: the SAME short-execution code the bake-off runs, in the
  agent runtime forward loop (F5b consistency).
- [ ] T8 — UI short surfaces (Live SHORT badge + signed qty + short P&L; leaderboard
  short-arm markers; forward-plan short-rule copy; disclaimers) — verify at the
  render-pixel layer per CLAUDE.md, with a negative control.
- [ ] T9 — The day-1 divergence e2e + the long-only byte-identity re-proof + the
  `verify_anchors.sh` 119/119 gate.

## Notes

Architect: see [`feature.md`](feature.md) § Engine-surface estimate for the candid
~5-8-dev-days-plus-ledger-plus-UI accounting, and § The five design forks for the
resolved-with-recommendation forks (which are operator-level vs architect-level).
