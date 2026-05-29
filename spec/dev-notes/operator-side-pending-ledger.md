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
## Done recipes (audit trail)

| Date surfaced | Recipe | Cost | Completed | Outcome |
|---|---|---|---|---|
| 2026-05-28 | **Bug #64 D.1.1 cold-cache Yahoo+Run** — cockpit Lab → Yahoo → SOL → Run; confirm ticking label + working Stop + no panic | ~5 min × 3 verify rounds | 2026-05-29 | **CLOSED — operator confirmed "it works" 2026-05-29.** Took 3 reactor-context recurrences to fully fix: (1) runner.rs:744 ticker — `rt.enter()` guard (attempt-3 `a87b5fa`); (2) runner.rs:395 `tokio::time::timeout` in fetch_with_backoff — guard-construct-drop (hotfix `61abef6`); (3) hyper-util DNS resolver `spawn_blocking` at dns.rs:119 — **durable fix: rt.spawn() the whole preload onto the tokio runtime** (`0298edb`). Architect Q1 "reqwest carries its own reactor" assertion was falsified twice; ADR-0050 D1 rewritten to spawn-don't-guard invariant. R2 Stop fixed via CancellationToken + fetch_join.abort(). Regression-guard hardening (production-call-through test) in flight to close tester INCONCLUSIVE. NOTE: Last30d won't have data in the 2026-system-clock environment (Yahoo has no future-dated bars); use 2024 ranges for real testing. |
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
- 2026-05-29 (developer): Bug #64 D.1.1 attempt-3 fix landed. Row updated FAILED → fix-in-flight. Feature: bug-64-d11-attempt-3-yahoo-run-runtime-context v0.1.0. Root causes: H-R1d (missing rt.enter()) + R2 structural omission (no cancel arm in preload select!). ADR-0050 codified. Operator re-verify recipe pending (cold-cache Yahoo SOL run + Stop-during-fetch manual test).
- 2026-05-29 (hotfix developer): Bug #64 D.1.1 attempt-3 HOTFIX. Operator cold-cache re-verify hit NEW panic at runner.rs:395 — `tokio::time::timeout` inside `fetch_with_backoff` without rt context. Architect Q1 assertion (fetch_with_backoff works without rt.enter() because reqwest spawns internally) FALSIFIED. Fix: `fetch_with_backoff` now takes `rt: &Handle`; all 3 tokio::time::* calls use guard-construct-drop pattern. New e2e test `lab_runner_cold_cache_fetch_e2e` (plain #[test]) proves fix. ADR-0050 § Changelog amended. Operator re-verify pending. Hotfix commit SHA: d1a7227.
