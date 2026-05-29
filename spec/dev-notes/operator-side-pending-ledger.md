---
slug: operator-side-pending-ledger
status: living
owner: orchestrator
updated: 2026-05-29
---

# Operator-side pending ledger

Persistent ledger of pending operator-run recipes that survives session
boundaries. Per the
`feedback_human_verification_recipe.md` operator-side persistence
contract (2026-05-29).

Maintained by the orchestrator: every time a recipe is surfaced to
the operator (per AGENT.md § Communication contract human-verification
6-section format), append a row here. When the operator pastes "done"
or "cancelled", update the row. Future sessions grep this file FIRST
to avoid re-surfacing stale items.

## Conventions

- Status values: `pending` / `done` / `cancelled`
- One row per recipe; append-only (mark status; never delete the row)
- Cost estimate format: operator-side wall-clock (build + interaction)
- "Unblocks" column names downstream agent work or feature progression

## Pending recipes

| Date surfaced | Recipe | Cost | Unblocks | Status | Notes |
|---|---|---|---|---|---|
| 2026-05-28 | **Bug #64 D.1.1 visual-verify** — `rm -rf data/yahoo/SOL-USD ; cargo run --release -p ui --bin cockpit_live` then Lab → Yahoo SOL → Run; watch label tick 0 / 1 bars · X.Xs every ~250 ms during 30-60 s cold-cache window | ~5 min | Closes Bug #64 D.1.1 attempt-2; full bug-log close pending operator confirm | **pending** | Recipe in bug-log #64 Attempt 2 entry; cockpit binary built at commit 8d38e38 |
| 2026-05-29 | **Yahoo bulk fetch v0.1.4** — `cargo run --release -p data --features yahoo-online --bin fetch_yahoo_klines -- --tickers BNB-USD,SOL-USD,XRP-USD,ADA-USD,DOGE-USD,AVAX-USD,DOT-USD,LINK-USD,MATIC-USD --interval 1d --start 2024-01-01 --end 2024-12-31` | ~3-8 min | lab-yahoo v0.1.4 M-DEV start gate (T-D1 evidence paste-in) | **pending** | Architect M-T1 closed at commit 539c990; M-DEV blocked on this |

## Done recipes (audit trail)

| Date surfaced | Recipe | Cost | Completed | Outcome |
|---|---|---|---|---|
| 2026-05-27 | Yahoo cache populate for BTC-USD 2024 1d (v0.1.1 prerequisite) | ~3 min | 2026-05-27 | Cache locked; revision SHA 7b33166e... ; v0.1.1 dev unblocked |
| 2026-05-27 | Yahoo cache populate for ETH-USD 2024 1d (v0.1.2 prerequisite) | ~3 min | 2026-05-27 | Cache locked; revision SHA e018f876... ; v0.1.2 dev unblocked |

## Cancelled recipes (audit trail)

| Date surfaced | Recipe | Cost | Cancelled | Reason |
|---|---|---|---|---|

## Changelog

- 2026-05-29 (orchestrator): file created per weekly-retro-2026-05-27-to-2026-05-29 fix-improve (c). Backfilled 2 pending + 2 done rows from session history. Going forward: every new operator-run recipe appends a row here at surface time.
