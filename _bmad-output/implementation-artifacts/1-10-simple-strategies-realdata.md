# Story 1.10: simple-strategies-realdata

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want sma / macd / rsi / bbands runnable on real Binance data in the Lab,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: sma / macd / rsi / bbands runnable on real Binance data in the Lab.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `simple-strategies-realdata` 0.2.0 - the base feature (presenter-done; leg formally closed by the 2026-07-26/27 code-review PASS below)

### Review Findings

<!-- bmad-code-review 2026-07-26 (first BMAD-native review; commits 93845af + c4a717d; layers: Blind Hunter, Edge Case Hunter, Acceptance Auditor — all completed).
     Gates re-run THIS session by the orchestrator: `ANCHORS PASS  (119 / 119)` · `spec-lint: PASS (0 violations)`.
     Fresh test re-run: backtest binance_cache_dispatch 12/12 real (no skip lines, --nocapture verified); realdata_simple_strategy_survey is #[ignore]'d (did NOT execute); ui lab_binance_* — see finding 1. -->

- [x] [Review][Decision] Compare/EquityCache key omits the data source — Binance and Synthetic/Yahoo runs of the same (strategy, symbol, range) shadow each other, newest report wins [crates/ui/src/lab/equity_loader.rs:89]. `LabTuple` = (strategy, symbol, range) only; report slug carries no source; the three-way toggle makes cross-source same-tuple runs a primary flow. Fix requires an intent call: key by source (report frontmatter `data_source:` exists since 93845af; pre-June reports lack it → treat as unknown) vs display-only surfacing vs record as stated limit.
- [x] [Review][Patch] **HIGH — ui real-data guard tests are vacuous on every machine (cwd-relative corpus root; any-Err→skip; no skip accounting)** [crates/ui/tests/lab_binance_divergence.rs:98; lab_binance_persist_compare.rs; lab_binance_render.rs; root cause crates/ui/src/lab/runner.rs:575]. EMPIRICALLY PROVEN 2026-07-26: corpus + `data/binance/REVISION.toml` present at workspace root, yet `--nocapture` shows `[skip] … REVISION.toml not found at data/binance/REVISION.toml` — cargo runs ui test binaries with cwd=`crates/ui/`. The backtest twin (cwd-compensated) ran its bodies for real. AC4(ui)/AC5/AC7 real-data halves have never executed under `cargo test`; the 2026-06-13 test report's "divergence tests ran for real" claim is true only for the 9 backtest tests (report is anchored — correction lives here + dev-note, never edit the frozen report). Fix: resolve corpus root via `env!("CARGO_MANIFEST_DIR")/../..` (or `set_current_dir` like binance_cache_dispatch.rs), skip ONLY on probe-NotFound, assert non-skip when the workspace-root probe exists.
- [x] [Review][Patch] Lab default range (Last90d) + Last30d permanently dead vs the 2023-24 pinned corpus, with a misdirecting "re-fetch the corpus" remedy; the two re-fetch hints also disagree [crates/ui/src/lab/runner.rs:592; crates/ui/src/lab/state.rs:90; crates/ui/src/strings.rs:1283]. Add an out-of-corpus-span early check with honest copy ("corpus spans 2023-01..2024-12 — pick H1/H2 2024 or a Custom 2023-24 range").
- [x] [Review][Patch] Feature-AC3 pin-assert clause unimplemented on the Lab path — loader verifies manifest self-consistency, never compares to the pin `3a8b96c4…` (CLI mirror does); `data/binance/REVISION.toml` is NOT git-tracked (no `.gitignore` exception, unlike every sibling corpus) [crates/ui/src/lab/runner.rs:663; crates/data/src/revision.rs:206; .gitignore:18]. Fix: pin-compare in the Lab loader + track the manifest.
- [x] [Review][Patch] Engine accepts non-Synthetic `data_source` with `bars_override: None` → runs synthetic GBM labeled "binance"/"yahoo" in written reports (latent API-boundary hole; Lab path always sets Some) [crates/backtest/src/engine.rs:1223; crates/backtest/src/scenarios/sma_composed_run.rs:423]. Fix: typed `UnsupportedDataSource` reject when non-Synthetic ∧ override None.
- [x] [Review][Patch] `spawn_lab_run` Binance production branch has zero automated coverage and the doc sells a nonexistent `binance_source_override` injection seam [crates/ui/src/lab/runner.rs:273; :1471]. Minimum: correct the doc; proper: add the seam + harness mirroring spawn_lab_run_yahoo_harness.rs.
- [x] [Review][Patch] Cache-miss `{window}` renders days-since-epoch integers (negative for pre-1970 Custom bounds) [crates/ui/src/lab/runner.rs:740]. Render YYYY-MM-DD.
- [x] [Review][Patch] Cadence badge is Yahoo-only — no 1h cue on Binance runs (SMA 20/50 semantics shift 60× vs 1m synthetic, unsignalled) [crates/ui/src/screens/lab.rs:355].
- [x] [Review][Patch] T-A1 label assertion is two decoupled `contains`, not the actual report row [crates/backtest/tests/binance_cache_dispatch.rs:137].
- [x] [Review][Patch] AC4 epsilon justification comment overstates margin 10× ("~10 bp" — it is 1 bp on a ~10_000 USDT book) [crates/ui/tests/lab_binance_divergence.rs:240].
- [x] [Review][Patch] Feature combo `binance` without `live` fails `-D warnings` (4 items cfg-gated `binance`-only; sole callers gated `live+binance`) [crates/ui/src/lab/runner.rs:574-737]. Widen gates to `all(live, binance)`.
- [x] [Review][Patch] Binance no-data conditions bypass the amber notice channel and the shared defensive arm ships Yahoo-branded copy under `target: "lab.yahoo"` [crates/ui/src/lab/runner.rs:996-1002; :1562].
- [x] [Review][Patch] Survey harness silently truncates on mid-stream `Err` (`while let Some(Ok(b))`) and `as u64` wraps negative timestamps [crates/backtest/tests/realdata_simple_strategy_survey.rs:58-66]. Loud break + partial-row marker. (Harness is `#[ignore]`'d — never gates.)
- [x] [Review][Patch] Loader-returned revision SHA is discarded by `spawn_lab_run` (`Ok((bars, _sha))`) while docs claim "report forensics" carry [crates/ui/src/lab/runner.rs:1472; :614]. Carry it or fix the doc (Yahoo-symmetric).
- [x] [Review][Patch] Story References line says trace `state=shipped`; trace.toml row is `presenter-done` — stale prose; self-heals at the done-flip (trace→shipped-terminal + CHANGELOG line CHANGELOG.md:117 already present) [_bmad-output/implementation-artifacts/1-10-simple-strategies-realdata.md:32].
- [x] [Review][Defer] AC8 no-binance two-chip proof unreachable by CI or any default invocation (`binance` in default features → file compiles empty; no `--no-default-features --features live` job) [crates/ui/tests/lab_source_toggle_no_binance.rs:30; .github/workflows/ci.yml:102] — deferred, CI-matrix scope → story 6-9 (cockpit-cross-platform CI shakeout, in-progress).
- [x] [Review][Defer] Label-match + range-mapper copy-paste fan-out (4 engine seams + 2 mappers + test dupes) [crates/backtest/src/engine.rs:1223 region; crates/ui/src/lab/runner.rs:426/:592] — deferred, pre-existing pattern; post-diff arms already adopted the three-way match (compile-enforced at enum seams).

## Dev Notes

- Source feature folder: `spec/v1/simple-strategies-realdata/` - frontmatter status **`presenter-done`** (verbatim), version `0.2.0`, updated `2026-06-17`.
- Status mapping: `presenter-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Strategy & backtest engine.
- Provenance: `git log -- spec/v1/simple-strategies-realdata` (full narrative); reports under `evidence/v1/simple-strategies-realdata/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-SIMPLE-STRATEGIES-REALDATA-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.
Review + close-out 2026-07-26/27: claude-fable-5 (orchestrator) + three parallel review subagents (Blind Hunter / Edge Case Hunter / Acceptance Auditor) + one dev subagent (terminated by session limit mid-pass; remainder completed by the orchestrator).

### Debug Log References

### Completion Notes List

### File List

#### Review close-out (2026-07-27, orchestrator)

All 14 patch findings + the resolved decision (D1 = key-by-source) APPLIED and
independently verified. Literal gate lines from THIS session's final battery:
`ANCHORS PASS  (119 / 119)` · `spec-lint: PASS (0 violations)` · fresh
`cargo clippy --workspace -- -D warnings` = 0 errors · P10 combo
`cargo clippy -p ui --no-default-features --features binance -- -D warnings` =
0 errors · `cargo test -p backtest` all green (14/14 dispatch incl. the two new
reject tests) · decisive no-skip proofs: `lab_binance_divergence` 3/3 in 0.13s
with the corpus loaded for real, `lab_binance_render` 3/3, AC5 round-trip
`PASS — … 4369 points, Compare cell built`, new `spawn_lab_run_binance_harness`
3/3. The `realdata_simple_strategy_survey` harness is `#[ignore]`'d and did not
execute (compile-verified only) — stated per the ignored-tests discipline.

**Discovered DURING the fix pass (the revived tests immediately caught three
more latent defects — all fixed this pass):**

- [x] [Review][Patch] Day-1 latent test bug: AC5 expected the companion CSV at
  `.with_extension("csv")` but the engine writes `<stem>-equity.csv` — the
  assertion was wrong on the day it was written and masked by the vacuous skip
  [crates/ui/tests/lab_binance_persist_compare.rs:170].
- [x] [Review][Patch] Engine lab write-seam hardcoded `btc-2023-1m-*` scenario
  names for all 9 arms regardless of symbol/range/source → Compare's loader
  scored 2024-preset requests 0 (NoReport) AND `delete_older_reports` made
  cross-source same-name reports replace each other on disk (file-level
  shadowing that D1's frontmatter filter alone could not fix). Fixed:
  `lab_scenario_name()` carries symbol + range + source; CLI/evidence path
  provably independent (`main.rs` never calls `run_scenario`; anchored bodies
  untouched — `ANCHORS PASS (119/119)` after) [crates/backtest/src/engine.rs].
- [x] [Review][Patch] `report::sma`'s template `\`-continuations elide leading
  whitespace → engine-written `strategy:` sub-keys are UNINDENTED → Compare's
  `parse_frontmatter` filed them top-level → `scan_one_root` silently skipped
  EVERY engine-written report (Compare cold-boot cells never appeared for
  sma-family lab runs). Writer bytes are hash-locked by the determinism anchors
  (`d2fa7616…` full-hash assert_eq) → fixed parser-side with a known-sub-key
  tolerant rule + regression test on the real unindented shape
  [crates/ui/src/compare/cache.rs].
- [x] [Review][Patch] `_audit_group_b_render` wrote debug PNGs to a
  non-guaranteed `/tmp/ui-audit/group-b/` dir — 5 spurious panics that
  fail-fasted the whole ui suite; `create_dir_all` added
  [crates/ui/tests/_audit_group_b_render.rs].
- [x] [Review][Defer] `report::sma` frontmatter template emits the unindented
  `strategy:` block (malformed vs the intended 2-space schema) — deferred:
  fixing the writer changes every freshly-rendered body and requires a formal
  determinism-anchor re-lock (ADR-0045 § D6 protocol); the tolerant parser
  covers all readers meanwhile.

**Standing-infra observation (NOT this story, proven unrelated by stash A/B —
identical failure sets at clean HEAD):** the macOS visual-baseline gate is
broadly red at HEAD on this machine (54 baseline-compare tests across 7
screens + 8 advisor-side render tests; the 2026-07-25 "8 drifts" flag was the
early tip). Environment-level rendering drift; routed to its own re-audit task.
