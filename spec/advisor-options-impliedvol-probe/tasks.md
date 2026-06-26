---
slug: advisor-options-impliedvol-probe
status: in-progress
owner: architect
updated: 2026-06-26
---

# Tasks — advisor-options-impliedvol-probe (DVOL implied-vol bake-off arm)

Sequenced developer checklist. Design: [feature.md § Design](feature.md#design-architect)
+ [ADR-0072](../architecture/adr/0072-dvol-implied-vol-exogenous-series-probe.md).
Every claim is grounded in code (file:line) in the design. **DESIGN IS LOCKED** —
the signal (`v0.dvol_regime`, W=30 daily, median cut), the arm id, the registration
seam (bake-off `v0.*` path, NOT cross_sectional), and the two day-1 gates are
pre-registered. No parameter search.

Frozen-gate / anchor invariants that hold for EVERY task below:
`write_report=false` (anchor-safe, 119/119), `classify_verdict` + bands FROZEN,
`v0.buyhold` benchmark unchanged, existing arms byte-identical.

---

## T1 — DVOL research spike (read-only diagnostic) — de-risk, does NOT block

- [ ] `crates/data/examples/dvol_diag.rs` — clone `crates/data/examples/basis_diag.rs`.
      Fetch (or read banked) BTC+ETH daily DVOL; compute the `v0.dvol_regime` signal's
      information content vs forward return: per-symbol time-series IC, cross-year
      sign-persistence, `--leak-check` (future-shifted DVOL must change the IC, cloning
      `basis_diag.rs:67-71`). Use ADR-0058 `PitSeries::as_of_value` for the as-of join
      (an f64 research clone is acceptable here, as `basis_diag.rs` does).
- [ ] NOT a bin, NOT anchored, throwaway. Report the IC numbers in the PR/handoff text.
- [ ] **Gate semantics:** T1 informs framing only. Zero IC → still ship the honest null
      (Fragile arm closes the vol channel). Build proceeds regardless.
- [ ] Emit a `watch -n 30 '<probe>'` block if the fetch/diag runs > 2 min.

## T2 — DVOL corpus + REVISION pin scaffolding

- [ ] Create `data/deribit-dvol/` corpus dir. Schema locked in feature.md § D-DVOL.1
      (`day_open_ts_ms` Int64, `day_close_ts_ms` Int64, `dvol_open/high/low/close` Float64;
      signal consumes `dvol_close` ONLY).
- [ ] Extend `.gitignore`: add `data/deribit-dvol/**/*.parquet` (mirror the
      `data/binance-basis` parquet-ignore rule). `REVISION.toml` stays TRACKED.
- [ ] Lay down `data/deribit-dvol/REVISION.toml` per the template in § D-DVOL.1
      (the `data/binance-basis/REVISION.toml` shape; `fetched_at` is the only clock,
      a metadata label, never hashed into an anchored body).

## T3 — The fetcher `fetch_deribit_dvol`

- [ ] `crates/data/src/bin/fetch_deribit_dvol.rs` — template clone of
      `crates/data/src/bin/fetch_binance_premium.rs`:
  - [ ] `DvolFetcher` trait (`async fn fetch(&self, url) -> Result<Vec<DvolCandle>>`) —
        every external I/O behind a trait (CLAUDE.md). Mirror `PremiumFetcher`
        (`fetch_binance_premium.rs:263`).
  - [ ] `HttpDvolFetcher` (reqwest) + `MockDvolFetcher` (test double, mirror
        `MockFetcher` `fetch_binance_premium.rs:661`).
  - [ ] `paginate_dvol(...)` — mirror `paginate_premium` (`fetch_binance_premium.rs:311`):
        follow Deribit's `continuation` / window by timestamp; stop on empty; keep
        in-window candles only.
  - [ ] `write_parquet` per `(symbol, year)` + `write_revision_manifest` (aggregate SHA)
        — mirror `fetch_binance_premium.rs:367`/`:566`. Deterministic + idempotent.
  - [ ] Endpoint LOCKED: `public/get_volatility_index_data`, `currency ∈ {BTC,ETH}`,
        `resolution=43200` (12h→daily close), `/public/` no-auth. Deribit API = primary;
        CryptoDataDownload CSV = corroboration/fallback (recorded in manifest metadata,
        NOT a second loader).
- [ ] Unit tests: paginator stops-on-empty + advances-cursor + filters-out-of-window
      (mirror `fetch_binance_premium.rs:711`+), all via `MockDvolFetcher`.
- [ ] Run the real fetch ONCE for BTC+ETH over 2023–2024; write parquets; pin the
      aggregate SHA into `REVISION.toml` (T2). Emit a `watch` block (the fetch is > 2 min).

## T4 — The loader + the as-of/leak-free join (`dvol_data.rs`)

- [ ] `crates/backtest/src/dvol_data.rs` — near-exact clone of
      `crates/backtest/src/basis_data.rs`:
  - [ ] `DvolDataSource { dvol_root, universe }` + `load(span, name)` — clone
        `BasisDataSource` (`basis_data.rs:124`/`:162`). Five steps: manifest-exists,
        per-parquet SHA, aggregate SHA vs `EXPECTED_DVOL_REVISION_SHA` (new locked const,
        clone `basis_data.rs:45`), parse `dvol_close` Float64→`Decimal`, filter span,
        sort `(day_close_ts_ms ASC, symbol ASC)`. **Refuse to run on unverified data.**
  - [ ] `DvolDataError` mirrors `BasisDataError` (RevisionMissing/Mismatch/Parse).
  - [ ] `dvol_as_of(series, bar_open_ts_ms) -> Vec<Option<Decimal>>` — verbatim clone of
        `basis_as_of` (`basis_data.rs:403`) over ADR-0058 `PitSeries::as_of_value`. LOCF,
        `None` warm-up, `Decimal` no-f64-roundtrip — all from `PitSeries`.
- [ ] **Port the no-look-ahead falsifier** (`basis_data.rs:553` `no_look_ahead_falsifier`)
      verbatim into `dvol_data.rs` tests (the JOIN layer leak-check).
- [ ] Port the supporting basis_data unit tests (out-of-span → None, empty → all-None,
      Decimal-precision-preserved) adapted to DVOL.

## T5 — The arm `DvolRegimeStrategy` (hand-written `Strategy`)

- [ ] `crates/strategy/src/dvol_regime.rs` — `DvolRegimeStrategy: Strategy` (hand-written;
      NOT a DSL `ComposedStrategy` — the DSL `Expr` `ast.rs:48` has no exogenous-series term).
  - [ ] `new(symbol, as_of_dvol: Vec<Option<Decimal>>, w: usize)` — holds the
        pre-resolved as-of vector; does NO joining itself (pure + unit-testable).
  - [ ] `on_bar`: bar-index cursor; ring of last-W DISTINCT daily closes (push only when
        the as-of close changes vs prior bar — dedups the 24× intraday forward-fill);
        `Decimal` median when ring full; `weight = 1 iff dvol_t < median, else 0`
        (tie→cash); emit `Buy` on 0→1-while-flat, `Sell` on 1→0-while-long. Warm-up
        (< W distinct closes) → weight=1 (HOLD = benchmark behavior, never look-ahead).
  - [ ] `on_tick` → no-op; `config_schema` → minimal stub. Long-only
        (`short_enabled=false`) rides the existing `sma_composed_run.rs:534` clamp.
- [ ] Unit tests: regime-classification truth table (calm→hold, stress→cash, tie→cash),
      warm-up→hold, distinct-daily-close dedup, median exactness (even W=30 = mean of
      15th/16th order stats), Buy/Sell transition edges.

## T6 — Bake-off registration seam (the `v0.*` path)

- [ ] `ScenarioConfig.dvol_override: Option<Vec<Option<Decimal>>>` new field
      (`crates/backtest/src/engine.rs:202`), default `None`. Mirrors
      `funding_override`/`basis_override` (`engine.rs:1057`). ALL existing
      `ScenarioConfig` literals add `dvol_override: None` (byte-identical).
- [ ] New `run_scenario` match-arm `"v0.dvol_regime" => { … }` (`engine.rs:945`+),
      structured like the `v0.obv` arm (`engine.rs:1767`): build `DvolRegimeStrategy`
      from `cfg.dvol_override` resolved against the run bars, register it, run the
      `sma_composed_run`-style bar-loop. `write_report=false` honored.
- [ ] `strategy_dir_slug("v0.dvol_regime") = "v0-dvol-probe"` new branch (`engine.rs:685`).
- [ ] `default_field()` (`crates/backtest/src/bakeoff/mod.rs:363`) += one line
      `StrategyId(SmolStr::new_static("v0.dvol_regime"))`. (Do NOT use
      `ScoreSource::DvolRegime`/`SweepFamily` — wrong machinery; see § D-DVOL.3.)
- [ ] Bake-off loop (`run_bakeoff` `bakeoff/mod.rs:688`): resolve `dvol_override` per run —
      if `req.symbol ∈ {BTCUSDT,ETHUSDT}` and `DvolDataSource::load` succeeds, compute the
      as-of vector vs preloaded bars and thread into the arm's `ScenarioConfig.dvol_override`;
      else **filter `v0.dvol_regime` out of `field`** (arm ABSENT, never crash —
      `dvol_supported(symbol)` predicate; § D-DVOL.6).
- [ ] Extend the `default_field_unchanged_additive_contract` test: assert `v0.dvol_regime`
      present + prior 9 ids unchanged in order.

## T7 — Day-1 gates (BOTH mandatory — CLAUDE.md non-negotiable)

- [ ] **(a) Divergence e2e** `crates/backtest/tests/dvol_regime_divergence_end_to_end.rs`
      (pattern: `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`). Synthetic
      bars + a DVOL series that crosses its 30d median ≥ once (flips weight 0↔1). Run
      `v0.dvol_regime` vs `v0.buyhold` on same bars+seed. **Assert
      `|equity_dvol − equity_buyhold| ≥ 1 bp` at final bar.** No-op arm ⇒ equal equities
      ⇒ test FAILS (catches the v3-vol-overlay no-op class).
- [ ] **(b) Arm-level leak-check** `crates/backtest/tests/dvol_regime_leak_check.rs`
      (pattern: `basis_data.rs:553` lifted to arm/equity level). Same fixture; build the
      arm with causal as-of DVOL vs the SAME series future-shifted +1 daily step.
      **Assert decision sequences (and equity) DIFFER.** Coincidence ⇒ leak ⇒ FAILS.
      (Plus the JOIN-layer falsifier already in T4.)

## T8 — Bake-off run + honest result

- [ ] Run the full bake-off on BTC (and ETH) over the frozen 2023–2024 window via the
      `backtest` skill, `write_report=false`. Capture the `v0.dvol_regime` verdict from
      the frozen `classify_verdict` gate vs `v0.buyhold`.
- [ ] Record the HONEST result. **A FRAGILE / null verdict is the expected, valid,
      shippable outcome** (honest coverage — closes the options/IV channel). Do NOT
      tune to escape Fragile (that voids the pre-registration).
- [ ] Verify a NON-BTC/ETH bake-off (e.g. SOLUSDT) completes cleanly with the arm ABSENT
      (D-DVOL.6) — no crash, leaderboard notes the BTC/ETH-only caption.

## T9 — Anchors + close

- [ ] `bash scripts/verify_anchors.sh` → **119/119** (before AND after — the arm runs
      `write_report=false`, additive only).
- [ ] `cargo fmt` + `cargo clippy --workspace -- -D warnings` clean; `scripts/precheck.sh`.
- [ ] (Optional, ship-time) If banking 1–2 DVOL coverage surfaces (BTC, ETH), do it via
      the standard ADR-0038 anchor-additive amendment — additive, never mutating. DEFAULT
      = 0 new anchors (exploratory `write_report=false` run). Orchestrator decides (OQ-4).
- [ ] Tester closes the loop with a `test-report.md`. Presenter assembles the deck after
      `VERDICT → PASS` (honest-coverage framing, null-is-valid).

---

## Sequencing notes

- **T1 first** (de-risk read), then the data spine **T2 → T3 → T4**, then the arm **T5**,
  then the wiring **T6**, then the two gates **T7** (developer MUST land both with the arm —
  not after), then **T8** (run + honest result), then **T9** (anchors + close).
- T5 (arm) depends only on T4's `dvol_as_of` signature, not on T3's real data — the arm is
  unit-tested against synthetic `as_of_dvol` vectors. So T5 + T7 can proceed in parallel with
  the real T3 fetch (which is the > 2 min network step).
- Frozen-gate + anchor invariants (top of file) are checked at T9 and must hold throughout.
