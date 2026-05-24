---
slug: lab-yahoo-realdata
version: 0.1.0
status: proposed
owner: analyst
updated: 2026-05-24
---

# Tasks — lab-yahoo-realdata

Wave-structured task list mirroring the project's standard pattern
(see `spec/lab-end-to-end-v2/tasks.md` for the canonical shape). Each
row carries an R-ref into [`feature.md`](feature.md) so the architect's
M-T1 decomp can fan out unambiguously.

## Wave A — Analyst (M0)

- [x] **T-A1** — Crate survey (yahoo_finance_api vs yfinance-rs vs
      yahoo-finance). Captured at feature.md § F1.
- [x] **T-A2** — Cadence-limit empirical survey. Captured at feature.md § F2.
- [x] **T-A3** — Strategy semantic-shift analysis (SMA / MACD / RSI /
      BBands defaults at hourly vs daily). Captured at § F3 + K3.
- [x] **T-A4** — Ticker convention options. Captured at § F4 + Q6.
- [x] **T-A5** — Cache-layout proposal (parquet, revision-pinned).
      Captured at § F5 + Q7.
- [x] **T-A6** — Lab UI surface outline for ui-designer. Captured at § F6
      + R-UI-1.
- [x] **T-A7** — Cross-feature impact: lab-end-to-end-v2 D-2c
      SUPERSEDED note. Captured at § F7; cross-feature spec edit
      tracked at T-A9.
- [x] **T-A8** — ADR-0040 outline (architect-authored at M-T1).
      Captured at § ADR-0040 outline.
- [ ] **T-A9** — Cross-feature spec edit: append SUPERSEDED note to
      `spec/lab-end-to-end-v2/feature.md`. Owner: analyst (this M0).
      Acceptance: the v2 brief reads "D-2c — SUPERSEDED 2026-05-24 by
      `lab-yahoo-realdata v0.1.0` per operator decision" and the
      changelog gains a same-day analyst entry.
- [ ] **T-A10** — Add backlog Active block. Owner: analyst (this M0).
      Acceptance: `spec/backlog.md` carries a new Active entry under
      Active naming `lab-yahoo-realdata v0.1.0` with the operator-decide
      Q's flagged.
- [ ] **T-A11** — Add `[[req]]` row to `spec/trace.toml`. Owner:
      analyst (this M0). Acceptance: row `REQ-LAB-YAHOO-REALDATA-001`
      exists with the minimum frontmatter shape per `.claude/agents/
      analyst.md` § Trace.toml.

## Wave B — Architect (M-T1) — gates on operator-decide Q1-Q10

- [ ] **T-AR1 — Q1 resolution.** Engine-dispatch shape. Analyst-
      recommended (b) source-agnostic engine; Lab swaps bars upstream.
      Acceptance: decomp.md records the picked shape + the file:line
      where the swap lands (e.g., `crates/ui/src/lab/runner.rs:spawn_lab_run`).
- [ ] **T-AR2 — Q2 resolution.** Asset universe scope. Analyst-
      recommended (a) crypto-mirror only. Acceptance: the 10-ticker
      `YAHOO_CRYPTO_UNIVERSE` slice contents are confirmed in
      decomp.md (operator may amend).
- [ ] **T-AR3 — Q3 resolution.** Yahoo crate pick. Analyst-recommended
      (a) `yahoo_finance_api 4.1.x`. Acceptance: exact version pin
      recorded in decomp.md; Cargo.toml dep planned.
- [ ] **T-AR4 — Q4 resolution.** Cadence policy. Analyst-recommended
      (c) adaptive. Acceptance: the `Cadence::derive(range)` table is
      finalised in decomp.md.
- [ ] **T-AR5 — Q5-Q10 resolutions.** All remaining Q's resolved with
      decomp.md citation.
- [ ] **T-AR6 — ADR-0040 authored.** File at
      `spec/architecture/adr/0040-yahoo-realdata-path-and-revision-pin.md`
      following the ADR-0032 template. Acceptance: ADR-0040 has a
      registered entry in the ADR README + a citation from the
      `spec/architecture/10-foundation-libraries.md` foundation
      libraries section.
- [ ] **T-AR7 — Module layout decision.** `crates/data/src/yahoo.rs`
      vs new `crates/data_yahoo` crate vs sub-module of
      `crates/backtest`. Acceptance: decomp.md records the file:line
      where the wrapper lands and how `--features yahoo` gates the
      compile.
- [ ] **T-AR8 — Wave plan.** Wave C (developer) split into independent
      sub-tasks per feature.md § Routes (Wave C-1 .. C-4). Wave D
      (ui-designer) gated on R-UI-1 acceptance. Acceptance: decomp.md
      carries a Gantt-style "do this in parallel; do this serially"
      table mirroring lab-end-to-end-v2's Wave D-1 .. D-4 shape.
- [ ] **T-AR9 — Anchor baseline gate.** `bash scripts/verify_anchors.sh`
      → `ANCHORS PASS (34 / 34)` re-confirmed at the architect ratification
      moment so the developer wave starts from a known-green baseline.

## Wave C — Developer (parallel; gates on Wave B)

Each row depends on the resolution of the Q's it cites. All four C-sub-
waves are independently shippable; T-C3 + T-C4 synchronise only on
`Venue::Yahoo`.

### Wave C-1 — `YahooBarSource` + parquet cache (no UI surface)

- [ ] **T-C1.1 — R1.1 / R1.2**: author `crates/data/src/yahoo.rs`
      (or new crate per T-AR7) exposing `YahooBarSource` + `Loaded` +
      `YahooError`. `--features yahoo` gated.
- [ ] **T-C1.2 — R1.3**: `YahooError` carries all variants from feature.md
      § R1.3 (network, 429, JSON parse, cache-miss, cadence-violation).
- [ ] **T-C1.3 — R2.1 / R2.2**: parquet writer + REVISION.toml writer.
      Layout per feature.md § F5.
- [ ] **T-C1.4 — R2.3**: revision-pin verifier on `load_cached`. Fails
      fast on SHA mismatch with the actionable error.
- [ ] **T-C1.5 — R2.5 / Q9**: `MissingData` at the operator-picked
      tolerance (analyst-recommended 95%).
- [ ] **T-C1.6**: round-trip integration test
      `crates/data/tests/yahoo_revision_verify.rs` against a fixture
      cache under `tests/fixtures/yahoo/`. No network.
- [ ] **T-C1.7**: network test `yahoo::tests::fetch_btc_usd_1d_last_30_returns_bars`
      gated `#[ignore]` or `#[cfg(feature = "yahoo-online")]`.

### Wave C-2 — `fetch_yahoo_klines` CLI

- [ ] **T-C2.1 — R2.4**: new bin under `crates/backtest/src/bin/`
      (or `crates/data/src/bin/`, per T-AR7). Args: `--ticker
      <X> --interval <1d|1h|1m> --start <YYYY-MM-DD> --end <YYYY-MM-DD>`.
      Idempotent.
- [ ] **T-C2.2**: exponential-backoff retry on `429` (architect ratifies
      cadence at T-AR3). K1 mitigation.
- [ ] **T-C2.3**: per-fetch Yahoo response checksum recorded in
      REVISION.toml metadata. K2 mitigation.
- [ ] **T-C2.4**: `--dry-run` flag prints the URL + expected bar count
      without writing.
- [ ] **T-C2.5**: integration test covers a fixture-replay run (no
      network).

### Wave C-3 — Lab dispatch + UI state

- [ ] **T-C3.1 — R3.1 / R3.5**: `LabState.source: LabSource` (new enum
      `Synthetic | Yahoo`). Default `Synthetic`. Toggle is no-op until
      operator presses Run.
- [ ] **T-C3.2 — R3.2 / Q1**: Lab runner swaps `synthetic_bars_hourly`
      for `YahooBarSource::load_cached` when `source = Yahoo`.
- [ ] **T-C3.3 — R3.3**: per Q1 resolution — either engine arm
      extension OR runner-side swap. Architect picks at T-AR1.
- [ ] **T-C3.4 — R3.4 / R-UI-1.2**: `RunState::Disabled` extends to
      cache-miss case with the actionable tooltip.
- [ ] **T-C3.5 — R4.1 / R4.2 / R4.3**: `YAHOO_CRYPTO_UNIVERSE` const;
      pair-chip row toggles on `source`.
- [ ] **T-C3.6 — R5.1 / R5.2 / R5.3**: `Cadence::derive(range)`; the
      cadence badge widget consumption.
- [ ] **T-C3.7**: integration test `crates/ui/tests/lab_yahoo_dispatch.rs`
      boots fixtures cockpit with a fixture Yahoo cache; asserts
      `LabRunCompleted(Ok(_))` for `BTC-USD + v0.sma + Last30d`.

### Wave C-4 — `Venue::Yahoo` variant cascade

- [ ] **T-C4.1 — K7**: add `Venue::Yahoo` to `trading_core::Venue`.
- [ ] **T-C4.2**: walk every `match venue` site under
      `crates/ui/`, `crates/audit/`, `crates/exec/`,
      `crates/backtest/`, `crates/strategy/` and add the missing arm
      (clippy `-D warnings` drives the list).
- [ ] **T-C4.3**: unit tests for the new variant: `Venue::Yahoo`
      `Display`, `FromStr`, `Serialize`/`Deserialize` round-trip.

## Wave D — UI-designer (parallel with Wave C-3)

- [ ] **T-D1 — R-UI-1.1**: `crates/ui/src/widgets/source_toggle.rs`.
      Lumen-tokenised. Gallery panels for both states.
- [ ] **T-D2 — R-UI-1.2**: cache-state badge widget. Lazy-reads
      `data/yahoo/REVISION.toml`. Gallery panels for present + missing.
- [ ] **T-D3 — R-UI-1.4 / R5.2**: cadence badge widget. Gallery panels
      for `Daily | Hourly | Minute`.
- [ ] **T-D4**: visual consistency review — toggle + badges harmonise
      with existing F10 disabled-run-button tooltips + the lab-end-
      to-end-v2 progress-bar (when it lands at v2's Wave D-4).
- [ ] **T-D5 — K8 mitigation**: panel-snapshot refresh planning;
      identify exactly which `panel_snapshots.rs` cases need re-emit.

## Wave E — Tester (M-FINAL)

- [ ] **T-T1**: `rust-build` (default + `--features yahoo`).
- [ ] **T-T2**: `rust-validate` (fmt + clippy `-D warnings` + docs +
      deny). Both feature sets.
- [ ] **T-T3**: `cargo test --workspace --lib` — 692 + new tests
      green; ignored network tests stay ignored.
- [ ] **T-T4**: `scripts/verify_anchors.sh` → `ANCHORS PASS (34 / 34)`
      — R-NR.1 gate.
- [ ] **T-T5**: `cockpit-smoke` skill — exit 0. R-NR.5 gate.
- [ ] **T-T6**: `uv run scripts/spec_lint.py` — exit 0. R-NR.4 gate.
- [ ] **T-T7**: H1-H6 evaluation:
  - H1 (Yahoo vs Binance equity divergence < 30%)
  - H2 (Yahoo fetch success > 95%)
  - H3 (100% cache-hit during Lab run)
  - H4 (parquet SHA deterministic across re-fetches)
  - H5 (default Lab UX byte-identical to pre-v0.1.0)
  - H6 (source-flip no rebuild).
  Record at `reports/test-final-2026-XX-XX.md`.
- [ ] **T-T8**: cockpit-performance idle-CPU regression check — ≤
      13.1% (R-NR.6).
- [ ] **T-T9**: integration test
      `crates/ui/tests/lab_yahoo_dispatch.rs` — PASS.
- [ ] **T_FINAL_VERDICT**: evaluator emits VERDICT in
      `reports/evaluation-2026-XX-XX.md` — PASS / FAIL / REGRESSION.
      Tester only ticks after evaluator PASS + verify-anchors PASS +
      cockpit-smoke PASS + spec-lint PASS.

## Wave F — Presenter (M-P1)

- [ ] **T-P1**: assemble
      `spec/lab-yahoo-realdata/presentations/lab-yahoo-realdata-2026-XX-XX.md`.
      Runs the live cockpit binary against a fixture Yahoo cache;
      captures Run → success screenshots; embeds H1-H6 numerical
      verdict; lists Q1-Q10 operator picks. Hand path back to operator
      for approval.

## Notes

- All sub-agents work on `main`; no worktrees, no branches.
- Sub-agents do NOT commit; the orchestrator owns
  `git add` + `git commit` + `git push origin main` at the end of each
  wave.
- Long-running tasks (cargo test on the full workspace, Yahoo fetches
  for the 10-symbol mirror) must emit a `watch -n 5 '<probe>'` block
  per the watch-recipe memo
  (`feedback_watch_recipe_for_long_running.md` in the user's project-
  memory store).
- Honest-tick rule applies: developer rows tick `[x]` only after
  citing (a) file:line, (b) test command, (c) test-output proving
  pass. Tester owns `T_FINAL_*` ticks.

## Changelog

- 2026-05-24 (analyst): initial task list. Waves A-F scaffolded;
  Wave A T-A1..T-A8 ticked (M0 deliverables). Wave B-F ungated
  pending operator-decide Q1-Q10 + architect M-T1.
