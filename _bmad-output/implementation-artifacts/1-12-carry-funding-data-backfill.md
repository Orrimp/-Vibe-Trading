# Story 1.12: carry-funding-data-backfill

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the funding-rate data backfill feeding the carry strategy,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `dev-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the funding-rate data backfill feeding the carry strategy.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

### Review Findings

<!-- bmad-code-review 2026-07-27 (burn-down 2 of 14; commit ab815d5; layers: Blind Hunter 13, Edge Case Hunter 12, Acceptance Auditor 3 — deduped to 15 below).
     Gates re-run THIS session: `ANCHORS PASS  (119 / 119)` · `spec-lint: PASS (0 violations)`.
     Auditor verified the delivered corpus BYTE-PERFECT in the current tree: 240/240 parquets match per-file SHAs; aggregate recomputes to the pinned `bf1ede44…`; consumers (funding_data.rs, param_robustness_sweep.rs) pin the same SHA. ALL substantive findings are FUTURE-RUN tool hazards (top-ups/re-fetches), not product-runtime or frozen-corpus defects. Independent cargo test -p data leg: pending at write time (polars cold build) — verdict withheld until it lands. -->

- [x] [Review][Patch] **HIGH — a future top-up can silently pin truncated or drifted months.** Three mutually-compounding holes: (a) completeness is never enforced at `--emit-revision-manifest` time — a mid-month `--end` writes a truncated `MM.parquet` indistinguishable from a full one and the manifest pins it; downstream SHA gates then prove *integrity of incomplete data* while the tool holds the completeness oracle (`expected_settlements_per_month`) unused at emit [crates/data/src/bin/fetch_binance_funding.rs:441-455, :506-520]; (b) `should_skip` is row-count-only — the pinned-SHA-aware hardening the sibling klines fetcher received in `8bcfa3a` (2026-06-16) was never back-ported, so content-drift with the right count skips silently [fetch_binance_funding.rs:343 vs fetch_binance_klines.rs:102-105]; (c) mid-month windows are never idempotent (expected = full month vs clamped fetch window → perpetual re-fetch) [fetch_binance_funding.rs:441-455].
- [x] [Review][Patch] Hardcoded 3 settlements/day breaks on Binance's symbol-specific 4h funding intervals — affected symbol-months mismatch forever (perpetual refetch); doc claims a `Returns None` conservative path that was never implemented [fetch_binance_funding.rs:170-182]. Moot for the frozen 2023-24 majors; guaranteed friction for top-ups.
- [x] [Review][Patch] Request hygiene: no `endTime` param (~10× over-fetch per month, ~240k rows downloaded to keep ~22k), the inter-page throttle is dead code in the only real path (months never paginate), and there is no retry/backoff — one 429/5xx/timeout aborts the entire multi-symbol backfill [fetch_binance_funding.rs:127, :215-222, :278-287].
- [x] [Review][Patch] Zero test coverage on the two riskiest seams: `should_skip` (the sole overwrite-vs-skip guard — 0 of 14 tests) and the `RawFundingRecord` camelCase serde wire mapping (mock sits above JSON; the only code touching the real wire format is untested) [fetch_binance_funding.rs:343, :103-110].
- [x] [Review][Patch] Shared `write_revision_manifest_with_tool` writes a vacuous manifest over an exists-but-empty root (empty `[files]`, the well-known empty-input aggregate `e3b0c442…`, verifies forever) — and the fn gained three more caller fetchers since, all untested [crates/data/src/revision.rs:155-192].
- [x] [Review][Patch] Non-atomic writes: parquet + REVISION.toml are truncate-in-place (no tmp+rename) — a crash mid-write leaves a corrupt file until the next run; abort between parquet rewrites and manifest emission leaves REVISION.toml pinning stale SHAs across the whole backfill window [fetch_binance_funding.rs:329-334; revision.rs:191-192].
- [x] [Review][Patch] Symbols neither trimmed nor validated, interpolated raw into URL query and filesystem path (`"BTCUSDT, ETHUSDT"` → `symbol=%20ETHUSDT` 400-aborts the run; `..` escapes `--out`; `&` rewrites the query) [fetch_binance_funding.rs:127, :421, :449-453].
- [x] [Review][Patch] Duplicate ingestion: in-window records extend `all` BEFORE the stale-data break, no dedup by `funding_time` — an overlapping server page persists duplicates this run [fetch_binance_funding.rs:264-279].
- [x] [Review][Patch] `end_date.next_day().unwrap_or(end_date)` silently drops the final requested day at `Date::MAX` instead of failing loud [fetch_binance_funding.rs:442].
- [x] [Review][Patch] Doc/assert hygiene: paginator doc claims a `< PAGE_LIMIT` stop that doesn't exist (and can never fire for funding-shaped data); committed "Actually with our logic…" reasoning residue; `calls.len() >= 2` asserted where logic guarantees exactly 3 (request-amplification regressions pass) [fetch_binance_funding.rs:241-243, :672-675].
- [x] [Review][Defer] Zero-record months leave no durable gap marker (transient warn only; one wasted request per run forever; manifest carries no coverage semantics) [fetch_binance_funding.rs:481-487] — deferred: gap/coverage semantics belong to a designed top-up story, not a drive-by. Owner: analyst (future funding top-up scoping). Revisit: epic-1 retrospective.
- [x] [Review][Defer] Schema/layout knowledge trapped in the binary (funding_data.rs re-hardcodes column names/path convention; no shared constant) [fetch_binance_funding.rs; crates/backtest/src/funding_data.rs] — deferred, refactor-when-touched. Owner: dev (next funding-schema touch). Revisit: epic-1 retrospective.

Dismissed as noise (1): the frozen brief names `write_revision_manifest()` where the implementation added `_with_tool()` — intent (shared aggregate-SHA algorithm) holds exactly; frozen-history nit. Also recorded, no action: stale test-comment residue (folded into the hygiene patch), 3/day-cadence caveat (folded into patch 2).

- [x] `carry-funding-data-backfill` 0.1.0 - the base feature (dev-done; leg formally closed by the 2026-07-27 code-review PASS)

## Dev Notes

- Source feature folder: `spec/v1/carry-funding-data-backfill/` - frontmatter status **`dev-done`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `dev-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Strategy & backtest engine (line added at the 2026-07-27 done-flip; was absent while dev-done).
- Provenance: `git log -- spec/v1/carry-funding-data-backfill` (full narrative); reports under `evidence/v1/carry-funding-data-backfill/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-CARRY-FUNDING-DATA-BACKFILL-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List

#### Review close-out (2026-07-27, orchestrator)

All 10 patch findings APPLIED (dev subagent killed by the session limit after
completing its edits; verification finished by the orchestrator) and
independently verified. Literal gate lines from THIS session: `ANCHORS PASS
(119 / 119)` · `spec-lint: PASS (0 violations)` · fresh `cargo clippy -p data
-- -D warnings` = 0 · `cargo test -p data` all green — the fetcher suite grew
14 → 146 tests (143 passed, 3 ignored network probes): completeness-at-emit
(`--allow-partial` escape, refusal lists offenders), pinned-SHA-aware
`should_skip` (klines `8bcfa3a` pattern back-ported, decision tree
documented), `[days*3, days*6]` cadence set, `endTime`-bounded requests +
bounded retry honoring Retry-After, tmp+rename atomic writes (parquet +
manifest), alphanumeric symbol validation, dedup-before-extend, `Date::MAX`
loud failure, doc/assert hygiene. Shared `revision.rs` gained the
exists-but-empty refusal + the `write_revision_manifest_with_tool` test; all
four caller fetchers unaffected (suite green). The frozen corpus + manifest
are byte-untouched (`git status data/` clean; anchors 119/119); the
frozen-corpus no-op property (a complete corpus skips every month) is proven
by the new tempdir skip-tree tests. Consumer proof: funding_data unit tests +
the `real_parquet_parses_to_expected_rows` real-corpus test run explicitly
(result quoted in the commit).
