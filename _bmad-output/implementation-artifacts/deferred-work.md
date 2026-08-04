# Deferred Work

Findings deferred out of code-review runs (bmad-code-review step-04 contract: one
heading per review, one bullet per finding). Deferred ≠ dismissed — each entry is
real, verified, and waiting for its actionable home.

**Discipline (2026-07-27):** every entry carries `Owner:` and `Revisit:` (a
concrete event, not a date-you'll-ignore). Each epic retrospective reviews this
ledger; an entry that survives two retrospectives unowned gets escalated to a
board action item or closed as accepted-limit — silently rotting here is not an
outcome.

## Deferred from: code review of 1-10-simple-strategies-realdata (2026-07-26)

- AC8 no-binance two-chip proof unreachable by CI or any default invocation: `crates/ui/tests/lab_source_toggle_no_binance.rs:30` is gated `cfg(all(feature="live", not(feature="binance")))` while `binance` sits in the ui crate's default features, so the file compiles to an empty test binary everywhere; `.github/workflows/ci.yml` has no `--no-default-features --features live` leg (and no `live,binance`-without-`yahoo` combo either). The two-chip contract is regression-unguarded since CI activation 2026-07-10. Home: story 6-9 (cockpit-cross-platform CI shakeout) — add a feature-combo job leg when the matrix stabilizes. Owner: orchestrator (story 6-9). Revisit: at story 6-9 close — the story cannot flip done without this leg decided (added or explicitly waived).
- Label-match + range-mapper copy-paste fan-out: the `data_source_str` three-way match is duplicated at four engine seams (`crates/backtest/src/engine.rs` sma/macd/rsi/bbands arms) and the H1/H2-2024 epoch constants + range→ms mapping are duplicated between the Yahoo (`crates/ui/src/lab/runner.rs:426`) and Binance (`:592`) mappers plus test files. Pre-existing pattern; the arms added after the reviewed diff already adopted the match (compile-enforced at enum seams), so consolidation is a refactor-when-touched, not a defect fix. Owner: dev (whoever next touches an engine arm or range mapper). Revisit: epic-1 retrospective.
- `report::sma` frontmatter template emits the `strategy:` sub-block UNINDENTED (the `\`-continuations elide leading whitespace) — malformed vs the intended 2-space schema, discovered when the revived AC5 test found Compare's scanner skipping every engine-written report (bug-log #66 A.4). The READER side is fixed (tolerant parser in `crates/ui/src/compare/cache.rs` + regression test); the WRITER fix changes every freshly-rendered report body and therefore requires a formal determinism-anchor re-lock per ADR-0045 § D6 (`d2fa7616…` full-hash assert_eq in `crates/backtest/tests/determinism.rs`). Do it in a dedicated re-lock pass, never as a drive-by. Owner: operator decision (re-lock touches determinism anchors). Revisit: epic-1 retrospective, alongside the visual-baseline re-audit (both are re-baseline-class work; batch them).

## Deferred from: code review of 1-12-carry-funding-data-backfill (2026-07-27)

- Zero-record funding months leave no durable gap marker: the fetcher warns transiently and writes nothing, so every future run re-probes the known-empty month and no loader can distinguish "no settlements" from "never fetched" (`crates/data/src/bin/fetch_binance_funding.rs:481-487`; the manifest carries no coverage semantics). Proper fix = coverage/gap semantics designed into the manifest, which belongs to a scoped funding top-up story. Owner: analyst (future funding top-up scoping). Revisit: epic-1 retrospective.
- Funding schema/layout knowledge trapped in the fetcher binary: `crates/backtest/src/funding_data.rs` re-hardcodes the column names, path convention, and rate-as-string decision with no shared constant — a schema evolution must land in ≥2 places. Refactor-when-touched. Owner: dev (next funding-schema touch). Revisit: epic-1 retrospective.

## Deferred from: code review of 1-13-monte-carlo-bootstrap-path-generator (2026-07-31)

- Resampler fabricates constant ±10bp wicks (real high/low geometry discarded): fine for today's config-driven fill logic, latent poison for any future intrabar fill/stop consumer — synthetic range geometry is systematically uniform unlike real data (`crates/data/src/synth/bootstrap.rs:410-411`; documented `synthetic_bars_hourly` convention, built for smoke paths, inherited by the credibility layer). Owner: architect — any future intrabar-fill story MUST revisit this before trusting MC fills. Revisit: epic-1 retrospective.
- GBM 3-copy dedup carve-out (D-C1.5 v0.2.0) still unbuilt and unowned: `momentum.rs::synthetic_bars_hourly`, `main.rs::synthetic_bars`, `determinism.rs::synthetic_bars_det` remain independent copies at HEAD; no story tracks the consolidation. Owner: dev (refactor-when-touched). Revisit: epic-1 retrospective.

## Deferred from: code review of 1-17-time-series-momentum-robustness (2026-08-04)

- Test-pollution hazard (from the c59998e commit-message admission, previously untracked): a test side-effect wrote 6 stray tcn-overlay reports into anchored evidence dirs during the 1-17 delivery and was cleaned by hand — the hazard class (tests writing into `evidence/**` instead of TempDir) recurs unowned. Fix shape: audit test write-paths for evidence/ targets; add a stray-file probe to `verify_anchors.sh` or CI (any `backtest-*`/`robustness-*` file in an anchored dir newer than its anchor row → loud warning). Owner: orchestrator (fold into the next CI/gates story or 6-9). Revisit: story 6-9 close or epic-1 retrospective, whichever first.
- Basket-gated warmup under per-asset TS mode (`all_warmed` requires ALL universe rings full — one late-listing/gappy symbol silently flattens the entire strategy forever; contradicts per-asset independence; latent trap for exactly the broader-universe route the anchored conclusion recommends). Behavior change → not a review drive-by. Owner: any future universe-expansion/TS-v2 story (MUST fix before broader universes). Revisit: epic-1 retrospective.
