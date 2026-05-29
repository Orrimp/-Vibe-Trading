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
| 2026-05-29 | **viewport-matrix T-VPM-D6 — 56 PNG visual eyeball** — `open crates/ui/tests/visual-baselines/` in Finder column-view + preview pane; focus `__floor` + `__operator` slots across 15 visual_snapshot fixtures + strategies_ready + chart_screen; also re-review the 3 regenerated Charts triple + 2 regenerated legacy `render_snapshots/*_dark_typical.png` | ~10-15 min | Closes T-VPM-D6; presenter v0.1.0 deck assembly unblocks | **pending** | 56 PNGs committed at ec79ac0; full punch list in viewport-matrix-v0.1.0 dev handoff envelope `[outputs]` |
| 2026-05-29 | **visual-fail-html-reporter v0.1.0 presenter approval** — open `spec/visual-fail-html-reporter/presentations/v0.1.0-2026-05-29.md` and the embedded HTML artifact at `spec/visual-fail-html-reporter/presentations/artifacts/v0.1.0-2026-05-29/sample-fail-emission.html`; tick the approval block | ~5 min | Closes v0.1.0 ship; merges to backlog Recent | **pending** | Presenter VERDICT → READY; presenter committed next |
## Done recipes (audit trail)

| Date surfaced | Recipe | Cost | Completed | Outcome |
|---|---|---|---|---|
| 2026-05-27 | Yahoo cache populate for BTC-USD 2024 1d (v0.1.1 prerequisite) | ~3 min | 2026-05-27 | Cache locked; revision SHA 7b33166e... ; v0.1.1 dev unblocked |
| 2026-05-27 | Yahoo cache populate for ETH-USD 2024 1d (v0.1.2 prerequisite) | ~3 min | 2026-05-27 | Cache locked; revision SHA e018f876... ; v0.1.2 dev unblocked |
| 2026-05-29 | **Yahoo bulk fetch v0.1.4** — 9 mid-cap tickers (BNB, SOL, XRP, ADA, DOGE, AVAX, DOT, LINK, MATIC) 2024 1d | ~3-8 min | 2026-05-29 | 11 ticker dirs total; 175 parquet files; REVISION.toml hash-locked. lab-yahoo v0.1.4 M-DEV unblocked (deferred to next session per operator pause directive). |

## Cancelled recipes (audit trail)

| Date surfaced | Recipe | Cost | Cancelled | Reason |
|---|---|---|---|---|

## Changelog

- 2026-05-29 (orchestrator): file created per weekly-retro-2026-05-27-to-2026-05-29 fix-improve (c). Backfilled 2 pending + 2 done rows from session history. Going forward: every new operator-run recipe appends a row here at surface time.
- 2026-05-29 (orchestrator): Yahoo bulk fetch v0.1.4 → done. 9 mid-cap tickers cached; REVISION.toml hash-locked. Bug #64 D.1.1 visual-verify remains pending.
