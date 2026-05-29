---
slug: operator-side-pending-ledger
status: living
owner: orchestrator
updated: 2026-05-29 (Bug #64 D.1.1 attempt-3 investigation linked)
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
| 2026-05-28 | **Bug #64 D.1.1 visual-verify** — `rm -rf data/yahoo/SOL-USD ; cargo run --release -p ui --bin cockpit_live` then Lab → Yahoo SOL → Run; watch label tick 0 / 1 bars · X.Xs every ~250 ms during 30-60 s cold-cache window | ~5 min | Closes Bug #64 D.1.1 attempt-2; full bug-log close pending operator confirm | **FAILED 2026-05-29** | Operator report: "endless spinning, no progress visible, cannot stop the running task." Progress label does NOT tick during cold-cache fetch AND Stop button non-functional. Bug #64 D.1.1 attempt-2 NOT fixed. Two regressions surfaced: (1) progress label dormant (original D.1.1 issue); (2) Stop button broken (NEW). Analyst investigation 2026-05-29 → [`bug-64-d11-attempt-3-investigation-2026-05-29.md`](bug-64-d11-attempt-3-investigation-2026-05-29.md). Recommended: architect M-T1 pass; OR cheap binary-freshness recipe first (H-R1a / H-R1b probe). |
## Done recipes (audit trail)

| Date surfaced | Recipe | Cost | Completed | Outcome |
|---|---|---|---|---|
| 2026-05-27 | Yahoo cache populate for BTC-USD 2024 1d (v0.1.1 prerequisite) | ~3 min | 2026-05-27 | Cache locked; revision SHA 7b33166e... ; v0.1.1 dev unblocked |
| 2026-05-27 | Yahoo cache populate for ETH-USD 2024 1d (v0.1.2 prerequisite) | ~3 min | 2026-05-27 | Cache locked; revision SHA e018f876... ; v0.1.2 dev unblocked |
| 2026-05-29 | **Yahoo bulk fetch v0.1.4** — 9 mid-cap tickers (BNB, SOL, XRP, ADA, DOGE, AVAX, DOT, LINK, MATIC) 2024 1d | ~3-8 min | 2026-05-29 | 11 ticker dirs total; 175 parquet files; REVISION.toml hash-locked. lab-yahoo v0.1.4 M-DEV unblocked (deferred to next session per operator pause directive). |
| 2026-05-29 | **visual-fail-html-reporter v0.1.0 presenter approval** — operator inspected deck + sample HTML artifact via AskUserQuestion | ~5 min | 2026-05-29 | APPROVED with all 3 open decisions accepted as carry-forward. v0.1.0 SHIPPED. |
| 2026-05-29 | **viewport-matrix T-VPM-D6 56-PNG visual eyeball** — operator opened `crates/ui/tests/visual-baselines/` in Finder | ~10-15 min | 2026-05-29 | PASS. "Images look fine." No layout breakage / clipping / blank canvas / font issues across all 56 PNGs (floor + typical + operator slots × 22 fixtures + 3 Charts + 2 legacy). Closes T-VPM-D6. |
| 2026-05-29 | **viewport-matrix v0.1.0 presenter approval** — operator approval via chat ("I approve the viewport matrix") | ~2 min | 2026-05-29 | APPROVED. 3 open follow-ups accepted as carry-forward (K3 cross-platform → ui-test-harness-ci Queue; render_snapshots legacy 1280×720 → v0.2.0 cleanup; T-VPM-D6 → CLOSED). v0.1.0 SHIPPED. |

## Cancelled recipes (audit trail)

| Date surfaced | Recipe | Cost | Cancelled | Reason |
|---|---|---|---|---|

## Changelog

- 2026-05-29 (orchestrator): file created per weekly-retro-2026-05-27-to-2026-05-29 fix-improve (c). Backfilled 2 pending + 2 done rows from session history. Going forward: every new operator-run recipe appends a row here at surface time.
- 2026-05-29 (orchestrator): Yahoo bulk fetch v0.1.4 → done. 9 mid-cap tickers cached; REVISION.toml hash-locked. Bug #64 D.1.1 visual-verify remains pending.
- 2026-05-29 (analyst): Bug #64 D.1.1 attempt-3 investigation dev-note linked from the pending row note column. Awaits operator decision on Q1 (scope), Q2 (label mechanism), Q3 (cancellation). Recommended path: Q1=(a) + Q2=(a)-with-recipe-first + Q3=(a) cancel-token wrap.
