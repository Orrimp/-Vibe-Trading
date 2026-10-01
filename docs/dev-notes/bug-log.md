---
slug: bug-log
status: living
owner: orchestrator
updated: 2026-05-25
---

# Bug log

Append-only ledger for the repo's local `#NN` bug counter. There is **no
GitHub Issues tracker** for this project — bug numbers live in commit
subjects + inline code comments (`Bug #63 — ...`). This file IS the
ledger.

## Conventions

- **Numbering** is monotonic and sequential. Skip a number if a draft
  PR was abandoned, but never reuse one.
- **Allocation**: the operator (or whoever opens the bug) picks the
  next free `#N` by checking `git log --oneline | grep -oE '#[0-9]+' | sort -unr | head -1`
  and adding 1.
- **Status values**:
  - `fixed` — landed on `main` via a tagged commit; no follow-up scope.
  - `partial-fix` — one or more sub-requirements shipped, others pending.
  - `open` — discovered but not yet fixed.
  - `wontfix` — investigated and explicitly closed without code change.
- **Status anchor**: every row links the **landing commit hash(es)**
  (and a feature folder when one exists).
- **Append-only**: do not rewrite past rows; if a `fixed` bug regresses,
  open a new `#NN` and link to the prior row in its body.

## Bugs

### `#54` — Lab Run errors invisible; cold-start tuple missing
**Status**: fixed
**Commit**: `799543a fix(lab): #54 Run errors now visible; cold-start = v0.sma × BTCUSDT`
**Area**: `lab-end-to-end-v2` (UI error surfacing + default scenario tuple).
**Notes**: Lab Run errors were swallowed by the runner; cockpit cold-start had no default strategy × pair. Now defaults to `v0.sma × BTCUSDT`.

### `#56` — Backtest config path not workspace-relative
**Status**: fixed
**Commit**: `47bb6d3 fix(backtest): #56 workspace-relative config path resolution`
**Area**: `crates/backtest` (path resolution).
**Notes**: Strategy TOML loader resolved paths relative to CWD instead of workspace root; broke when binaries ran from sub-crate dirs.

### `#57` — D-2.5 per-pair filter missing for cross-sectional Lab runs
**Status**: fixed
**Commit**: `cb065fa feat(lab) + cleanup: #57 D-2.5 per-pair filter; #58 trace state alignment + audit dev-note`
**Area**: `lab-end-to-end-v2` (cross-sectional chart filter).
**Notes**: Top-10 momentum / pairs runs showed all 10 symbols' fills + bars overlapped on the chart. Per-pair filter (operator picks BTCUSDT, sees only BTC's data) ships D-2.5.

### `#58` — Trace state misaligned vs spec rows; audit dev-note gap
**Status**: fixed
**Commit**: `cb065fa` (same commit as #57)
**Area**: `spec hygiene` + `audit`.
**Notes**: `spec/trace.toml` rows out of sync with feature.md frontmatter; companion audit dev-note describing the cleanup.

### `#60` — Audit P2/P3 sweep
**Status**: fixed (ops sweep)
**Commit**: `a7f9b10 ops: #60 audit P2/P3 sweep`
**Area**: spec hygiene (P2 / P3 audit findings cleanup).
**Notes**: Mechanical bookkeeping pass over outstanding audit P2/P3 items. Not a code bug — used the #NN counter for traceability.

### `#61` — `lab-yahoo-realdata` v0.1.1 anchor scaffolding
**Status**: partial-fix
**Commit**: `b78cf97 feat(lab-yahoo): #61 v0.1.1 partial anchor — commit REVISION.toml + scaffold test`
**Area**: `lab-yahoo-realdata` (Yahoo anchor lock).
**Notes**: Test scaffolding + REVISION.toml committed. Final Yahoo anchor lock blocks on operator populating the cache (`cargo run -p data --features yahoo,yahoo-online --bin fetch_yahoo_klines`). Tracked as `lab-yahoo-realdata v0.1.1` row in [`spec/backlog.md ## Active`](../archive/pre-bmad-spec/backlog.md).

### `#62` — `lab-polish-round-2`: position curve + SMA param editor + KPI density
**Status**: partial-fix (R2 + R3 shipped; R1 in flight)
**Commits**:
- `091c3e9 analyst(lab-polish-round-2): #62 author v0.1.0 spec — position curve + param editor + KPI density`
- `c1cddbe feat(backtest): #62 R2 backend — SmaComposedRunInput.sma_{fast,slow}_len overrides`
- `ae26281 feat(lab): #62 R2 UI — SMA fast/slow param editor`
- `371d870 feat(lab): #62 R3 KPI strip densification — 8 cards in 2×4 layout`

**Area**: `lab-polish-round-2`.
**Notes**: Three operator workflow gaps surfaced after `lab-end-to-end-v2` shipped — R1 position-curve overlay, R2 SMA param editor, R3 KPI strip density. R2 + R3 in `main`; R1 in flight. Feature [`spec/lab-polish-round-2/feature.md`](../archive/pre-bmad-spec/v1/lab-polish-round-2/feature.md).

### `#63` — Cross-sectional Stop + progress wiring dead; Yahoo fetch could freeze cockpit
**Status**: fixed
**Commit**: `982830f fix(lab): #63 cross-sectional Stop+progress wiring + Yahoo fetch timeout`
**Area**: `lab-end-to-end-v2` follow-up (`crates/backtest/scenarios/` + `crates/ui/src/lab/runner.rs`).
**Notes**: Two regressions left behind by `lab-end-to-end-v2 v0.1.0`:
1. Cross-sectional scenarios (`momentum`, `pairs`, `tcn_overlay`) never took `cancel_rx` / `progress_tx`. Stop button silent; progress bar frozen. Fix threads both through and polls at the 128-bar boundary (`bar_idx.trailing_zeros() >= 7`). CLI passes `cancellation_pair()` with handle alive + `ProgressSender::disabled()` so anchored output is byte-identical by construction.
2. `runner.rs::fetch_with_backoff` had no per-attempt timeout — a hung Yahoo endpoint could freeze the cockpit indefinitely. Added 60 s `tokio::time::timeout` per attempt; retries with backoff up to `max_retries`.

### `#64` — Progress bar stuck for short runs (Yahoo daily Last30d, narrow custom ranges)
**Status**: fixed
**Commit**: _pending — same commit as this row's authoring_
**Area**: `lab-end-to-end-v2` follow-up (`crates/backtest/scenarios/` × 4 + `crates/ui/src/lab/runner.rs`).
**Discovery**: Operator-reported. Synthetic-hourly runs showed smooth animation; Yahoo daily Last30d (~30 bars) stayed visually stuck.

**Root cause** (diagnosed via `tracing::warn!` probes — see `git show 88ea755~..HEAD -- crates/ui/src/lab/progress.rs` for the temporary instrumentation). All 4 scenarios used a sparse poll boundary calibrated for hundred-to-thousand-bar runs:
- SMA path: `bar_idx & 0x1F == 0` (warmup, every 32) → `bar_idx & 0x7F == 0` (steady, every 128).
- Cross-sectional (Bug #63 wiring): `bar_idx.trailing_zeros() >= 7` (every 128).

For a 30-bar Yahoo daily Last30d run, only `bar_idx = 0` hit the boundary. One progress event fires with `current_bar=0, total_bars=30` → `progress_pct = 0/30 = 0.0` → bar renders at empty fill, never advances before the engine completes in milliseconds. The Yahoo preload phase between channel creation and engine start additionally rendered the 30% indeterminate fallback during the network/disk await.

**Fix** (two parts):
1. **Always emit at the final bar** regardless of poll boundary. In all 4 scenario files (`sma_composed_run.rs`, `momentum.rs`, `pairs.rs`, `tcn_overlay.rs`), the gate now reads `<existing boundary> || bar_idx == total_bars.saturating_sub(1)`. For 30-bar runs this gives 2 emits (bar 0, bar 29) → bar visibly advances 0% → 97% → done. For 720-bar synthetic Last30d hourly the existing 9 emits become 10 — no regression.
2. **Yahoo preload sentinel** — `crates/ui/src/lab/runner.rs::spawn_lab_run` now emits a `Progress { current_bar: 0, total_bars: 1, elapsed_ms: 0 }` event BEFORE the `preload_yahoo_bars` await. The widget renders this as 0% with the label `"0 / 1 bars · 0.0s"` — an explicit pre-engine state instead of the silent indeterminate fallback.

**Anchor contract**: Progress events are channel-only, never written to report bodies. 34/34 anchors stay byte-identical.

**Probes used during diagnosis** (now reverted): `tracing::warn!` at `crates/ui/src/bin/cockpit_live.rs:1200` (LabRunRequested handler) + `crates/ui/src/lab/progress.rs::Recipe::stream()` (entry + rx_opt = Some/None branch). Captured to `/tmp/cockpit-probes.log` via `RUST_LOG=lab.progress.recipe=warn`. Probe log showed salt bump 1→2→3 across runs with `rx_opt = Some` every time — ruling out the iced subscription as the failure mode.

**Re-investigation 2026-05-27** (orchestrator-spawned post-operator-revisit): operator reported the bar still appears "stuck" on Yahoo runs after this fix shipped. Investigation agent `a4e18698810fa3d4b` confirmed the original fix is **intact** at HEAD — all 4 force-emit gates + the Yahoo preload sentinel still in place. Verdict: **D — UX artifact, not a code regression.** Two residual artifacts:

- **D.1** Cold-cache Yahoo fetch shows the sentinel `Progress { current_bar: 0, total_bars: 1, elapsed_ms: 0 }` static for 30-60 s during network/disk fetch — visually indistinguishable from stuck (no label tick during fetch).
- **D.2** Post-preload engine runs in ~10-100 ms; the two emits (~0% → ~99%) compress into a single repaint frame before `LabRunCompleted` clears `run_progress = None` and the bar vanishes. Synthetic feels smoother because no preload pause + 720-bar SMA loop spans multiple repaint frames.

Dev-note with full 11-hop code-path trace + 3 scoped fix options (not applied; operator-decide) at [`docs/dev-notes/bug-64-progress-bar-investigation-2026-05-27.md`](archive/2026-Q2/bug-64-progress-bar-investigation-2026-05-27.md). Includes operator repro recipe in the new AGENT.md 6-section format. Fix options:
- **D.1.1** sentinel ticker (~25 LoC, runner.rs) — emit periodic sentinel updates during preload
- **D.1.2** dedicated preload-status field (~50 LoC, 3 files — flagged out of scope)
- **D.2.1** post-completion linger (~25 LoC, 2 files) — hold the 99% bar visible for 500 ms before clearing

Operator picks which (if any) to apply.

**Attempt 1 — D.1.1 + D.2.1 applied 2026-05-28, REVERTED same day** (commit `5f9f920` → revert at `05937e4`):

The developer agent `a115c172c99353fdd` shipped both fixes with all unit gates green (411 → 415 PASS; 70/70 anchors; clippy clean). However operator visual-verify against a real cold-cache Yahoo run surfaced **three regressions**:

1. **No label visible at all** — the existing `"0 / N bars · Xs"` label that was working before D.1.1 stopped rendering, suggesting `LabState::run_progress` no longer reaches `Some(...)` during the preload window.
2. **Progress bar stuck at ~30%** — this is the iced indeterminate-state fallback that the original Bug #64 fix specifically eliminated via the pre-engine sentinel emit at `runner.rs:617-621`. The 30% reappearing implies the new `tokio::select!`-based ticker either dropped the sentinel or the channel was broken by the refactor.
3. **Stop button does nothing after Run** — likely caused by the D.2.1 changes to `LabRunCompleted` / `LabRunProgressDone` no longer clearing `run_progress`. Stop's handler path probably checks `run_progress.is_some()` to gate enablement, but the linger keeps it `Some` until either timer expiry OR the linger-id mismatches — and Stop doesn't increment `progress_linger_id` (only `LabRunRequested` does).

Lesson: **the dev's unit gates (415 PASS) DID NOT catch regressions on the live cockpit channel.** Adding 4 LabState invariant tests proved the new state-machine logic locally but missed the interaction between LabState's run_progress lifetime + the actual `progress_tx` channel flow in `spawn_lab_run` + the Stop button's gating predicate.

Disposition: bug stays `fixed` (per the original 2026-05-25 commit `<unknown SHA>` — bar advance + sentinel both worked pre-attempt). D.1.1 / D.2.1 polish remains an open follow-up if operator wants to re-attempt with deeper testing (suggested: live cockpit smoke + iced-test driver covering Stop-after-Run + a sentinel-emission unit-test asserting `progress_tx.send` actually fires before `preload_yahoo_bars().await`).

**Attempt 2 — D.1.1 applied 2026-05-28, harness-gated** (commit `<pending>`):

The lab-recipe-test-harness shipped at commit `d4fc321` (ADR-0048) provided the structural gate missing from attempt 1. D.1.1 was re-implemented with two critical bug fixes over attempt 1:

1. **Sentinel fires FIRST** (before ticker creation, before first `ticker.tick().await`). Attempt 1 called `ticker.tick().await` BEFORE the sentinel emit, delaying first event by ~250ms. Fix: sentinel emit happens unconditionally as the first statement in the YahooCache block.

2. **Preload future pinned once** (`std::pin::pin!`). Attempt 1 called `preload_yahoo_bars(&cfg, &range)` inside the `select!` loop body, creating a NEW future each iteration — preload never made progress. Fix: create + pin the future ONCE before the loop; each `select!` iteration polls the same pinned future to completion.

**D.2.1 status**: NOT implemented in this attempt; **operator-DROPPED 2026-05-28** after harness-conflict surfaced. The Surface 2 harness (lab_stop_button_gating.rs Test 1, line 134) mandates `cockpit.lab_state.run_progress.is_none()` immediately after `LabRunCompleted`. The D.2.1 linger approach (keeping `run_progress` alive) directly contradicts this. Since the harness IS the gate and cannot be modified to pass the implementation, D.2.1 would require either (a) a separate `linger_progress` field with view-layer changes, or (b) a harness update to accommodate the linger semantics. Operator chose option (none) — D.1.1 alone closed the primary visual complaint (cold-cache stuck at 30%); D.2.1 was always polish for the SECONDARY fast-run flash issue, and not worth the architect cycle or the harness softening. **D.2.1 is closed as won't-fix**, not deferred.

**D.1.1 file:line citations**:
- `crates/ui/src/lab/runner.rs:718–807` — sentinel emit + `std::pin::pin!` preload future + `tokio::select!` ticker loop (production `#[cfg(feature = "yahoo")]` path only; mock path left unchanged to keep Surface 1 tests passing).

**Test evidence**:
- Surface 1 (`spawn_lab_run_yahoo_harness.rs`): 3/3 PASS
  - `sentinel_fires_before_preload_await` — first event < 50ms (sentinel before ticker)
  - `channel_survives_after_preload` — channel alive after preload completes
  - `ticker_events_stop_after_preload_complete` — zero ticker-leak events after preload
- Surface 2 (`lab_stop_button_gating.rs`): 3/3 PASS
  - `full_lifecycle_ok_completion_clears_inflight` — run_progress = None after LabRunCompleted
  - `err_completion_clears_inflight` — error path also clears inflight
  - `stop_requested_mid_run_leaves_inflight_true` — Stop press doesn't flip inflight prematurely
- K5 (`cockpit_training_pressed_wiring`): 5/5 PASS
- `cargo test -p ui --lib --features live`: 411/411 PASS
- `bash scripts/verify_anchors.sh`: 70/70 PASS

**Harness earned its keep**: Test 1 (`sentinel_fires_before_preload_await`) directly falsifies attempt 1's regression A (50ms gate vs attempt 1's ~250ms delay). Test 3 (`ticker_events_stop_after_preload_complete`) would catch ticker-leak regressions. The harness confirmed no regression was reintroduced before handoff.

**Operator visual-verify recommendation**: STILL RECOMMENDED for the preload animation UX (the harness confirms no channel regressions but does not exercise the production `#[cfg(feature = "yahoo")]` ticker path directly — that path requires a live Yahoo cache miss to observe). The harness is sufficient to gate channel correctness; visual UX smoothness requires operator confirm on a cold-cache run.

### `#65` — `vol_killswitch_overlay` is a no-op (computes counters, never mutates Signal.kind)
**Status**: FIXED 2026-05-26 — Q4=(p3) "Both" — fix test fixture AND broaden overlay filter.
**Discovery commit**: (Wave 1 parent commit — overlay-e2e test found the no-op)
**Fix commit**: (vol-killswitch-overlay-noop-fix v0.1.0 developer pass 2026-05-26)
**Recovery feature**: [`spec/vol-killswitch-overlay-noop-fix v0.1.0`](../archive/pre-bmad-spec/v1/vol-killswitch-overlay-noop-fix/feature.md) (P0; developer pass complete 2026-05-26)
**Area**: `crates/strategy/src/vol_killswitch_overlay.rs`, `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs`.
**Root cause** (H1 REFUTED by architect M-T1 probe): the ORIGINAL bug report hypothesized the `sig.symbol == bar.symbol` filter was too narrow. H1 was REFUTED. The REAL root cause was the TEST FIXTURE warmup gap: `MomentumStrategy`'s ring buffer (capacity = `lookback_minutes + 1 = 61`) never filled because only ~31 bars per symbol were fed — ring buffer never filled → inner strategy never emitted signals → overlay had nothing to mutate.

**Fixes applied** (Q4=(p3) "Both"):
- A.1 (test fixture): `lookback_minutes` 60→5 in `stub_momentum()` (capacity 61→6). Flat BTC warmup prices prevent GARCH early-kill with `min_median_floor=1e-3`.
- A.2 (broadened filter): dropped `if sig.symbol == bar.symbol` guard; kill now converts ALL basket signals to Hold, not just the triggering symbol's signals.
- A.3: Removed `#[ignore]` annotations; added `broadened_filter_dampens_cross_sectional_basket` test; 4/4 tests pass.
- A.4: This entry.

**Test evidence** (all 4 tests green after fix):
```
test post_trigger_signals_are_hold ... ok
test broadened_filter_dampens_cross_sectional_basket ... ok
test passthrough_when_threshold_unreachably_high ... ok
test trigger_fires_and_equity_diverges ... ok
test result: ok. 4 passed; 0 failed; 0 ignored
```

**Overlay hygiene gate**: `vol_killswitch_overlay` removed from `KNOWN_UNCOVERED` allowlist (2/2 gate tests pass).

**Why this matters**: analyst's framing in `docs/dev-notes/testing-strategy-review-2026-05-25.md` — "a killswitch that doesn't kill is the worst kind of no-op." Risk profile: in production, if vol exceeds the killswitch threshold, the strategy continues trading as if nothing happened. This is the worst-case failure mode for a risk-overlay.

### `#66` — ui real-data guard tests vacuous since day 1 (cwd-relative corpus root); revival exposed 3 more latent defects
**Status**: FIXED 2026-07-27 — story 1-10 code-review pass (the first BMAD-native review).
**Discovery**: 2026-07-26, Edge Case Hunter finding empirically confirmed by the orchestrator: with the corpus AND `data/binance/REVISION.toml` present at the workspace root, `cargo test -p ui --test lab_binance_divergence -- --nocapture` printed `[skip] … REVISION.toml not found at data/binance/REVISION.toml` — cargo runs ui test binaries with cwd=`crates/ui/`, so the cwd-relative `BINANCE_CORPUS_ROOT` resolved to nowhere; `try_load_binance_bars` mapped ANY `Err` to a skip-as-pass. The backtest twin (`binance_cache_dispatch.rs`) compensates cwd and genuinely ran.
**Impact**: the AC4(ui)/AC5/AC7 real-data halves of `lab_binance_{divergence,persist_compare,render}.rs` never executed under `cargo test` on ANY machine since 2026-06-13; the anchored test report's "divergence tests ran for real (not skipped)" claim was true only for the 9 backtest-side tests (report is byte-immutable per ADR-0038 — this entry + the story's Review Findings are the correction of record).
**Area**: `crates/ui/tests/lab_binance_{divergence,persist_compare,render}.rs`, `crates/ui/src/lab/runner.rs`, `crates/backtest/src/engine.rs`, `crates/ui/src/compare/cache.rs`.
**Fixes applied**:
- A.1: tests resolve the corpus from the workspace root (`CARGO_MANIFEST_DIR`-derived, mirroring the backtest twin), skip ONLY on genuine probe-absence, and FAIL loudly when the corpus is present but the loader errors. Proof: 3/3 divergence in 0.13s with zero skip lines, corpus loaded for real.
- A.2 (revival catch 1): AC5's companion-CSV assertion expected `.with_extension("csv")`; the engine writes `<stem>-equity.csv` — day-1 latent test bug, fixed.
- A.3 (revival catch 2): the engine's lab write-seam hardcoded `btc-2023-1m-*` scenario names for all 9 arms → Compare resolution scored 2024-preset requests 0 AND cross-source same-name reports replaced each other on disk. Fixed via `lab_scenario_name()` (symbol+range+source tokens); CLI/evidence path provably unaffected (`main.rs` never calls `run_scenario`; anchors 119/119 after).
- A.4 (revival catch 3): `report::sma`'s `\`-continuation template emits the `strategy:` sub-keys UNINDENTED → `compare::cache::parse_frontmatter` filed them top-level → `scan_one_root` silently skipped EVERY engine-written report. Writer bytes are determinism-hash-locked (`d2fa7616…`) → tolerant-reader parser fix + regression test on the real shape; writer re-emission deferred to a formal ADR-0045 § D6 re-lock.
- A.5: this entry.
**Moral** (same class as #65 and the v3-vol no-op): a test that exists and passes is not a test that runs — skip paths need positive proof of execution (non-zero runtime, no-skip assertion when the fixture is present), and the 2026-06-13 tester recorded vacuous passes as "ran for real". The review's revival of one test chain surfaced three real product bugs within the hour.

### `#67` — Research-harness lanes priced cross-symbol fills at the trigger bar's close (anchored C2/C3 evidence = execution-artifact noise)
**Status**: OPEN — fix + formal re-lock owned by story 1-25-harness-fill-correctness-relock (CRITICAL; one program with 1-24). Disclosure entry, 2026-07-31.
**Discovery**: story 1-14 code-review Blind Hunter finding, orchestrator-verified same day at HEAD.
**Mechanism**: `PaperEngine::step` prices EVERY order in a batch at the stepped bar's close (no `order.symbol == bar.symbol` check, `crates/backtest/src/paper.rs:118-136`); `scenarios/montecarlo.rs::run_path` steps cross-symbol momentum rebalance batches against the single trigger bar (`:274+`), so a BTC order can fill at ADAUSDT's ~$0.25 close (mispricing factors 1.5e-5×..3.6×). The v0.1.1 solvency guard discards mispriced-EXPENSIVE buys; mispriced sells still bank wrong proceeds. Same pattern in `threshold_sweep.rs::run_cell` (which also lacks the Bug-B solvency guard entirely).
**Blast radius** (extended 2026-07-31: anchor #86 confirmed contaminated via run_path — a SECOND route besides run_cell; BUYHOLD clean. Extended 2026-08-03 by the 1-16 review: anchor #87 confirmed via the identical chain. Extended 2026-08-04 by the 1-17 review: anchors #90/#91 (TS θ-surfaces 2023+2024, `c1bf9325…`/`ff7e7dda…`) confirmed end-to-end; the 1-18 horizon surfaces #92-#99 run the same lane — flagged for that review. BUYHOLD rows clean throughout): the anchored RESEARCH-evidence class — the C2 harness distribution (81% median MaxDD / P(loss) 75.2% / compressed Sharpe band feeding the FRAGILE verdict) and the C3 threshold-sweep FAMILY-UNIFORM-FRAGILE lanes. **NOT the advisor gate**: `bakeoff/bootstrap.rs` resamples log-returns from candidate equity curves and never re-executes fills (verified) — crowns, verdicts, and the era-qualified ship-passive thesis stand on the bakeoff gate independently.
**Riders in the same re-lock**: √8575-vs-√8760 annualization constant; hashed-body WEAK/MARGINAL vocabulary vs the frozen 5-signal FRAGILE rule; sentinel-zero metric pooling; negative-final Calmar NaN; slippage-blind solvency pre-flight; FILL_SEED domain separation; decorative exposure cap.
**Moral** (the #65/#66 lineage continues): the harness's e2e gates validated a synthetic stand-in reducer, never the production fan-out — a fill-arithmetic corruption of this size sailed through a VERDICT→PASS tester run because no test ever priced a real cross-symbol fill. The 1-14 review pass re-points those gates at the real chain.

### `#68` — The θ-grids' drift/hold-band swept axis is behaviorally INERT (anchored narratives attribute results to a lever that does not exist)
**Status**: **RESOLVED 2026-09-25 — implemented, not dropped.** The operator confirmed that
"implemented + binding-tested" satisfies story 1-26's AC1 entry gate: *"Zählt als erfüllt."*
The axis stays. Implemented 2026-08-23 (`723ca742`). Disclosure entry, 2026-08-03.

**The premise of this entry is void, verified 2026-09-25.** #68 and #69 were ONE defect: the
sole implementation reading BOTH the exposure cap and the drift band
(`risk::size_portfolio_target`) had no production caller. Wiring it for #69 on 2026-08-23
made the drift axis live as a side effect. The code says so itself
(`montecarlo.rs:238`): *"until the sizer was wired nothing consumed the third axis
(bug-log #68)"*. Its binding test is green —
`portfolio_controls_bind.rs::drift_band_suppresses_resizes_that_a_tight_band_performs`,
which holds everything constant and varies only `drift_rebalance_threshold` 0.001 vs 0.90.

**Why this went back to the operator rather than into a commit.** The 2026-08-19 ruling
recorded in story 1-26 AC1 is *"#68 dropped (drift axis removed from the grid and from every
surface that presents it as an explored dimension)"*. That ruling was made while the axis was
inert — **four days before the wiring landed**. Executing it would have deleted a working,
binding-tested dimension. Implementing was the other half of this entry's own
"implement-or-drop", so the outcome sat inside the ruling's intent — but 1-26's gate reads
the letter. **Ruled 2026-09-25: the letter is satisfied.** The narrative correction at
regeneration still stands: the anchored θ-surface narratives (#86, #87) attribute cell
results to a hold band that was inert WHEN THOSE SURFACES RAN, and 1-26 re-prices them with
it live. The axis's third-dimension claim becomes true only for the new rows.

Original entry below, unchanged as the record of what was found.
**Discovery**: story 1-16 code-review Blind Hunter H1; orchestrator-accepted on the reviewer's caller-graph evidence.
**Mechanism**: `drift_rebalance_threshold` reaches exactly three places — the config hash, the report grid-definition table, and `MomentumStrategy.drift_threshold` marked `#[allow(dead_code)]` (written, never read). No drift/hold-band logic exists in `run_path`/`PaperEngine`; the only real implementation (`risk::size_portfolio_target`) has zero production callers; the equal-weight open/close-on-membership signal scheme cannot express a hold band. 0.10 vs 0.30 vs 0.50 changes nothing.
**Impact**: the anchored θ-surface narratives (#86 momentum, #87 MR) attribute cell results to "wide hold-band / low-turnover engineering" — a confounded interpretation (lookback+k carry everything). Verdicts stand (all-FRAGILE is direction-preserving under an inert axis); the INTERPRETATION and the grid's third-axis scientific claim do not. No test could go red on this: no cell pair is drift-only distinct.
**Moral** (the #65 lineage, again): a parameter that is hashed, printed, and swept is not a parameter that is EXECUTED — the same class as the v3-vol no-op overlay (#65), one layer up. Sweep design must include a per-axis "this axis moved the output" probe (a drift-only cell pair would have caught this on day 1).

### `#69` — `portfolio_exposure_cap` is INERT engine-wide; D-TSM.2's ratified safety premise is false; the anchored TS surfaces ran ~2× the documented gross exposure, alphabetically rationed
**Status**: **FIXED 2026-08-23** (`723ca742`), verified 2026-09-25. The corrected exposure
description + thesis re-affirmation still ride story 1-26's regeneration.

**Verified at source 2026-09-25**, because the entry sat at OPEN for a month after the fix
landed and 1-26's entry gate reads this line. `scripts/callers.sh size_portfolio_target` now
reports a PRODUCTION caller — `crates/backtest/src/scenarios/montecarlo.rs:520`, inside
`run_path` — where the census in this entry found zero. The binding test AC1 asks for exists
and is green: `crates/backtest/tests/portfolio_controls_bind.rs::gross_cap_refuses_the_whole_rebalance_and_the_count_is_surfaced`.

Original entry below, unchanged as the record of what was found.
**Discovery**: story 1-17 code-review Blind Hunter H-1 (the #68-mandated caller-graph probe, aimed at the risk-limit layer).
**Mechanism**: `Order::new` validates ONLY `per_symbol_exposure_cap` (`crates/core/src/order.rs:123-170`); `portfolio_exposure_cap`'s sole implementation (`crates/risk/src/portfolio.rs:189`) has zero production callers; `run_path` sets `Some(dec!(0.50))` decoratively with empty per-order Position snapshots. Invisible in every top-K family (K≤5 × 10% fixed-fraction ≤ 50% — the cap could never bind).
**Impact**: story 1-17's LOCKED design certified fixed-fraction sizing as safe BECAUSE "the 0.50 portfolio cap throttles" high-cardinality bars — false. TS long/flat emits up to 10 Buys → ~90-100% gross in high-breadth regimes (anchored tim 0.78-0.87) vs the hashed `held_constant | exposure_cap=0.50` row in anchors #90/#91; the only real limiter is the cash pre-flight, which rations ALPHABETICALLY (BTreeMap emission order), starving alphabetically-late symbols — violating the design's own per-asset-independence criterion. The FAMILY-UNIFORM-FRAGILE verdict likely survives (p5-Sharpe ≈ exposure-scale-invariant; margins ≥~1.06 Sharpe uniform), but prob_loss/p95-maxdd are exposure-sensitive banded signals and the anchored body misdescribes the book — the 1-25 regeneration must correct the description and re-affirm the closure explicitly.
**Moral** (the #68 lineage, risk-limit edition): a limit that is set, printed, and ratified into a design's safety argument is not a limit that is ENFORCED. Every declared risk control needs a binding test (construct a scenario where the limit must bind; assert it does).

> **SOURCE-CONFIRMED 2026-08-17 (story 1-25 work, orchestrator).** The mechanism is now
> pinned exactly, and it is stronger than "inert": the enforcer is **test-only**.
>
> `RiskLimits.portfolio_exposure_cap`'s own doc says *"When `Some(cap)`,
> `risk::size_portfolio_target` enforces the cap atomically across the entire rebalance
> vector."* That function is the **sole** enforcer — `Order::new` never reads the field.
> Census of `size_portfolio_target(` across the whole workspace:
>
> | site | kind |
> |---|---|
> | `risk/src/portfolio.rs:67` | the definition |
> | `risk/src/portfolio.rs:269,293,317,350` | its own unit tests |
> | `agent/tests/v1_rebalance_reject.rs:88,169,255` | a test file |
> | **anywhere in production** | **ZERO** |
>
> `montecarlo.rs` = 0 calls. `param_robustness_sweep.rs` = 0 calls. So the sweep scenarios
> set `portfolio_exposure_cap: Some(dec!(0.50))` (`threshold_sweep.rs:156`,
> `tcn_overlay_weights.rs:148`, `patchtst_overlay_weights.rs:155`; `pairs.rs:155` sets 0.75),
> that number is printed into the hashed report bodies, and **nothing ever reads it**.
>
> **The sharp part:** `v1_rebalance_reject.rs` PROVES the enforcer works. So the corpus
> contains a passing test for a limit that production never invokes — a gate verifying a
> function rather than a behaviour, which is bug-log #77's shape one level out. Compare #80,
> where the *helpers* turned out production-dead; here the *enforcer* is, so the limit
> evaporates while its test stays green.
>
> **Family:** this is the third declared-limit-with-no-reader found in two days — #85 (two
> account loss stops, zero read sites), #71 (a per-symbol cap that capped the order instead
> of the position, and could be walked past in increments), and now #69. AC3's
> "enforce-or-delete + a BINDING test for **every** declared risk limit" is the right
> response precisely because the pattern is systemic rather than incidental.

### `#70` — The R3 data-coverage gate compares COARSE expected against RAW loaded: on horizon lanes it passes with ~4% of the data
**Status**: FIXED 2026-08-04 (story 1-18 review patch pass) — one-line unit correction + a horizon-invariance test.
**Discovery**: story 1-18 code review (Acceptance Auditor H2 + Edge Hunter H2, independently); orchestrator-verified at source.
**Mechanism**: `param_robustness_sweep.rs` derives `bar_count` from `(year, horizon)` — 2190 at 4h, 365 at daily — then sets `expected_total = bar_count * symbols.len()` and hands it to a loader that reads the **1h** parquet corpus and counts RAW hourly bars. The R3 tolerance is a one-sided lower bound (`loaded < ceil(expected*995/1000)` → bail), so at `--horizon daily` the gate demanded 3,632 bars against 87,600 actually loaded: **a corpus missing 95.9% of its hours would have passed silently.** The in-code comment asserted the opposite ("the coverage check above stays on the 1h count") and the architect's D-HR.2 promised it verbatim.
**Impact**: anchors #92-#99 were produced under a coverage gate that could not bind. The revision-SHA pin was the only real provenance check that held.
**Moral** (the #69 lineage, units edition): #69 was a limit that was never *enforced*; this is a limit that was enforced against **the wrong unit**. A gate whose two sides are computed in different units is not a gate. Every threshold comparison needs its units asserted — ideally in a test that feeds a deliberately-deficient input and demands the bail.

### `#71` — `Order::new`'s exposure cap is SIDE-BLIND: it rejects position-CLOSING sells, silently, exactly when risk is highest
**Status**: OPEN — owned by story 1-25 (AC3 risk-limit correctness). Disclosure entry, 2026-08-04.
**Discovery**: story 1-18 code review (both hunters independently); orchestrator-verified at `crates/core/src/order.rs:160-170`.
**Mechanism**: the check is `notional / current_equity > per_symbol_exposure_cap` with **no `side` term** and no use of the passed position snapshot. A `Side::Sell` liquidating a long worth more than 40% of equity is rejected identically to a Buy that would create one. The call site (`scenarios/montecarlo.rs`) is `if let Ok(ord) = Order::new(..) && let Ok(fills) = engine.step(..)` — **no else arm, no `warn!`, no counter** — so the dropped exit is invisible in every report.
**Impact**: reachable wherever a leg appreciates or its siblings crash (the horizon surfaces run p95 max-drawdown 75-93%). When it fires, the strategy's internal `held_symbols` believes the position closed while the engine's book keeps it open forever — every subsequent decision is computed from a false flat. Blast radius is strictly larger than #67's: it changes **which orders exist**, not just their price.
**Process note — the part that matters** (corrected 2026-08-04: *every* "developer", "architect" and "tester" in this repo's history is an AI agent under a prompt; there is no human author to whom judgment can be attributed): the 1-18 delivery pass *recorded this exact behaviour* in a test comment ("the risk guard silently rejects the SELL … leaving the physical position open forever") and then shipped a **gentler fixture** (`build_1h_up_down_bars_moderate`, +0.1%/bar instead of +1%/bar) so the cap would not trip. The defect was observed, written down, and worked around — and nothing in the harness required it to become a bug entry. That is a HARNESS hole, not a personal lapse: the rule that must exist is "a fixture weakened to keep a test green is a finding, and the weakening must be justified in the story or refused".
**Moral**: a risk cap that bounds *order notional* instead of *resulting exposure* is pointed backwards — it blocks de-risking precisely when de-risking is what the book needs. And when a test has to be softened to stay green, the softening is the finding.

### `#72` — The bootstrap's cosmetic 1-hour timestamp ladder makes every time-based rule horizon-blind (carry lanes harvested ¼ to 1/24 of their funding)
**Status**: CODE FIXED 2026-08-04 (`bar_span_hours` supplied explicitly; settlement boundaries counted over the real span; gate `funding_accrual_scales_with_declared_bar_span`). The anchored surfaces still need the 1-25 re-run + re-lock, and the durable claim stays qualified (`evidence/v1/horizon-retest-robustness/reports/ERRATA-2026-08-04.md`).
**Discovery**: story 1-18 code review (Edge Hunter C1 + Blind Hunter C1, independently); orchestrator-verified at source AND empirically in the anchored bodies.
**Mechanism**: `BlockBootstrapPathGen` re-stamps every generated bar as `epoch_base + Duration::hours(i)` with `tf: Timeframe::OneHour` **hardcoded**, whatever the source bars' real cadence. Funding settlement is then detected as `hours_since_epoch % 8 == 0` — which on a resampled path counts **bars, not hours**. At 4h that is one settlement per 32 real hours; at daily, one per 8 real days instead of three per bar.
**Empirical fingerprint** (g=0, 2023, per path): 1h **15,490** → 4h **3,039** → daily **267**.
**⚠ CORRECTED 2026-08-04 (see #73):** the ratios drawn from these numbers ("~1/4", "~1/24") are WITHDRAWN. The 1h figure is itself inflated by the #73 multiplicity defect (~universe size), so it was never a valid baseline. The cadence defect described below is real and is fixed; its *magnitude* relative to truth is established by the 1-25 re-run, not by these numbers.
**Impact**: anchors #96-#99 measured a carry strategy with most of its only edge deleted, and their hashed bodies assert "even at the native settlement cadence…" — a cadence those runs never simulated. The carry × coarse-horizon leg of the thesis closure is UNRESOLVED pending re-lock (the TS legs are unaffected). Any future time-based rule (funding, session boundaries, calendar effects) inherits the same blindness.
**Moral**: a synthetic timestamp ladder is a modelling *convenience* that silently becomes a modelling *assumption* the moment any rule reads real time from it. Generated paths must either carry their true cadence or every time-derived rule must take the cadence explicitly — never infer it from a stamp the generator invented.

### `#73` — Funding accrued once per SYMBOL-BAR instead of once per settlement: every carry surface over-accrued by ~the universe size
**Status**: FIXED 2026-08-04 (same pass as the #72 cadence fix) — dedup by timestamp + two measurement-derived regression gates.
**Discovery**: found by the orchestrator while designing the #72 fix, by questioning the review's own framing: the accrual block sits inside `for bar in &merged_bars`, and `ReplayFeed::merge_synthetic` interleaves EVERY symbol's series sorted by (ts, symbol), yet the block was gated only on the bar's timestamp with no per-timestamp dedup.
**Measurement, not inference** — hold exactly ONE position, vary only the universe size (i.e. the number of bar events sharing each timestamp), keep prices/rates/settlements identical:

| universe | realized funding |
|---|---|
| 2 symbols | −5.0 |
| 3 symbols | −7.0 |
| 4 symbols | −9.0 |

Funding that depends on how many *other* symbols exist is not funding. (The offset — 2N+1 rather than a clean multiple — comes from warmup and per-bar rebalancing; the *dependence on N* is the defect.)
**Impact**: the anchored carry surfaces run a 10-symbol universe, so **every** carry lane over-accrued by roughly an order of magnitude — including the 1h anchors (#88/#89), which the story-1-18 review had described as the *correct* reference. They are not a reference.
**CORRECTION OF RECORD**: bug-log #72 reported that carry-4h harvested "~1/4" and carry-daily "~1/24" of true funding, citing per-path totals 15,490 → 3,039 → 267. **Those ratios were computed against a contaminated 1h baseline and are withdrawn.** The honest statement: funding accrual was wrong at *every* horizon by a factor combining universe size (over) and bar-span blindness (under); no carry surface in the corpus measured the funding a carry strategy would actually earn. The corrected magnitudes come from the 1-25 re-run, not from arithmetic on the old numbers.
**Fix**: `last_accrual_ts` collapses a timestamp's symbol-bars to ONE accrual. Gate: `funding_accrual_is_invariant_to_universe_size` (the experiment above, inverted into an assertion).
**Moral**: when a rule lives inside a loop, ask what the loop is actually iterating. "Once per bar" and "once per instant" are different statements whenever bars are multi-symbol — and the difference is invisible in every aggregate the report prints.

### `#74` — The mandated AD-16 divergence gate was satisfied by a test that cannot fail: the signal was injected through a channel that moves equity by itself
**Status**: FIXED 2026-08-11 (story-1-20 review patch pass) — the falsifiers re-pointed at the production wiring, plus a `trades > 0` companion assertion.
**Discovery**: derived independently by two review layers (Blind Hunter H2, Edge Case Hunter H4) and verified at source by the orchestrator against the committed revision.
**The mechanism** — and why this one is not just "another vacuous test" (#66): the basis e2e falsifiers inject the signal via `funding_override`, which `run_path` uses for **two** unrelated purposes — it feeds the strategy's score map *and* it is the accrual channel. So the injected map settles as 8-hourly cashflow at ±0.5-2% of notional, roughly **60× the 1 bp epsilon the assertions test**. The difference the gate measures therefore comes from the accrual, not from the signal. Destroy the signal completely — return `Some(Decimal::ZERO)` instead of `Some(-mean)` — and the suite stays green, because the two compared runs still carry different cashflows.
**The test said so itself.** The committed helper carries this comment, verbatim:
> `// The accrual block will run but its effect is minor for these selection tests.`

The effect was asserted to be minor and never measured. It was ~60× the threshold.
**Compounding it**: `r_br_baseline_equity_divergence` compared the basis arm against a *price* baseline with no `trades > 0` companion — so an arm that never trades sits at exactly its initial capital and "diverges" from a compounding baseline by far more than 1 bp. A dead arm passes the liveness gate by being dead.
**Why it matters more than its severity suggests**: AD-16 — "every strategy overlay or sizing-modifier ships with a baseline-equity-divergence e2e from day 1" — exists *because of* the v3-vol-overlay no-op (#65). This is that exact failure class, reproduced **inside the gate written to catch it**. The non-negotiable was honoured in form and void in substance, and it stayed that way through a VERDICT→PASS.
**Confirmed by mutation, three ways** (temporary mutations, each observed, each reverted):

| mutation | old suite | new suite |
|---|---|---|
| `basis_reversal_score` → always `None` (the #65 no-op class) | **6/6 green** — basis arm pinned at 100 000 while the price baseline compounds, so `\|Δ\| > 10` passes | RED: *"the basis arm executed 0 fills — it never traded"* |
| preserve-branch → unconditional `with_funding(funding_override)` | **all 5 `funding_override`-wired tests green** | RED: *"got 0 fills, i.e. the arm never traded"* |
| `Some(-mean)` → `Some(mean)` (sign flip) | green — the two equities merely **swap** (100 000 ↔ 136 161), so a symmetric `\|Δ\| > 1` cannot see it | RED on selection **order** |

The middle row is the finding in one line: under the revert, the five tests wired through the *test* channel all stayed green while only the production-wired test failed.
**Fix**: falsifiers drive the production wiring (strategy pre-loaded via `with_funding`, `funding_override: None`, matching `param_robustness_sweep.rs`), assert the arm actually traded, and assert the *direction* of selection so a sign flip goes RED.
**Moral**: a divergence test proves nothing unless the signal reaches the strategy through the **same channel production uses**. If the test channel has any independent effect on the measured quantity, the test measures the channel, not the signal — and "its effect is minor" is a measurement, never an assumption. Ask of every gate: *which* difference is this assertion actually seeing? (Now the ninth mandatory probe — the **channel probe** — in `review-playbook.md` § 4.)

### `#75` — `run_path` overwrites the pre-injected SCORE map with the ACCRUAL map: the market-neutral BASIS arm silently ran the FUNDING score, and a headline scientific closure rests on the artifact
**Status**: OPEN (code fix + re-run are anchor-impacting → story 1-25). Disclosure and record-correction issued 2026-08-11 with the story-1-21 review.
**Severity**: the highest of the lineage so far — it does not distort a number, it **silently substitutes one experiment for another** and the substitution was then read as a scientific result.

**The mechanism.** `MomentumStrategy` uses ONE field, `funding_map`, for two semantically different things: the **score** sidecar and the **accrual** sidecar. The MN lane needs both — basis for the score, real funding for the short-leg cost — so the sweep driver injects the basis map via `.with_funding(basis_map)` and passes the funding map as `TcnScenarioInput::funding_override`. Then `run_path` does:

```rust
let funding_map_for_accrual = funding_override.clone();
let mut strategy = if let Some(map) = funding_override {
    strategy.with_funding(Some(map))   // ← REPLACES the pre-injected basis map
} else {
    strategy                            // ← preserve-branch: only when override is None
};
```

`with_funding` is `self.funding_map = funding;` — a full replacement. For the MN lane `funding_override` is always `Some`, so the basis map is **always** clobbered. The driver's own comment states the intent it does not achieve: *"Basis → score (via with_funding, BasisReversal arm). Real funding → accrual (via funding_override)."*

And `basis_reversal_score` and `carry_score` are the **same function modulo comments** — same `funding_map`, same shared `funding_rings`, same lookback, both returning `−mean`. Fed the same map they are bit-identical. So `mn-basis` ≡ `mn-funding`, exactly.

**Confirmed empirically with a control** (orchestrator, at source and in the anchored bodies):

| pair | numeric differences |
|---|---|
| `mn-basis` vs `mn-funding` (0bps 2023, 0bps 2024, 5bps 2023) | **ZERO** — bodies differ only in title, `score_source=` label, and one prose word |
| **control:** `mn-basisperp` vs `mn-funding` (0bps 2023) | **every cell** — p50 −0.064156 vs +0.013327, liquidations 328 vs 148, p95_maxdd 100.00% vs 99.78% |

The control is what proves the mechanism: `mn-basisperp` routes its basis through a **different field** (`basis_score_map`, via `with_basis_score`), so it escaped the overwrite and produced genuinely different numbers. Perp basis and funding correlate ~+0.47/+0.66, not +1.0 — two real series cannot produce bit-identical output across 200 bootstrap paths including identical integer liquidation counts.

**What this invalidates**: anchors **#108-#111** (`mn-basis`) are duplicate funding runs, not basis evidence. The pre-registered **k2** kill-criterion ("if arm 1 ≈ arm 2, the basis IS the funding mirror") fired on a wiring artifact. **R-MN.6**, the three-arm confound resolver that was the feature's headline requirement, delivered **two** distinct arms. The claim *"the derivatives-positioning domain is CLOSED with finality"* does not follow: the market-neutral basis spread — the exact thing the story was built to test — never ran.

**What SURVIVES, stated so the correction is not over-read**: `mn-basisperp` (the basis⊥funding residual) genuinely consumed the basis and came back negative (p50 −0.064/−0.043, 100% p95 MaxDD, 328/210 liquidations across 200 paths). So real evidence stands that the basis carries no orthogonal alpha *as a residual*. The closure is **weakened and re-scoped, not annihilated** — but it can no longer be stated as final, and it was never era-qualified as the thesis requires.

**Why it survived every gate**: the story's own falsifier suite sets `funding_override: None` in its harness, so **every test takes the preserve-branch while production takes the overwrite branch** — the test channel differs from the production channel in precisely the way that hides the defect. That is bug-log #74's mechanism, one story later, now in production rather than in a test.

**Process note of record (mine).** I reviewed these exact lines hours earlier during the story-1-20 review, wrote about the preserve-branch, and had a test built for it — but only ever asked what happens when `funding_override` is `None`. I never asked what happens when it is `Some` *and* a map is already injected. The **channel probe** I added to the playbook that same session is exactly the probe that catches this. A probe you write and do not then apply to the neighbouring lane is not yet a habit.

**Fix (1-25)**: give the score sidecar and the accrual sidecar **separate fields** so neither can clobber the other, then re-run the `mn-basis` arm and re-derive k2, R-MN.6 and the closure language. A guard asserting "never overwrite a non-None score map" is the minimum; separate channels are the real fix.
**Moral**: one field serving two meanings is a silent substitution waiting for the first caller that needs both. When two callers write the same field for different reasons, the second write is not a configuration — it is a bug with a delay fuse. Name the channels apart.

### `#76` — The basis⊥funding RESIDUAL arm ranks the basis axis INVERTED relative to its own specification: it longs the HIGHEST basis
**Status**: OPEN (fix + re-run are anchor-impacting → story 1-25). Found by the story-1-21 Edge Case Hunter; chain re-verified at source by the orchestrator.
**Why it compounds #75 into something worse**: #75 established that `mn-basis` (#108-#111) never saw the basis. #112-#115 are funding by design. That leaves **#116-#119 (`mn-basisperp`) as the only anchored MN surfaces that consumed the basis at all** — and they consumed it backwards. **No anchored MN surface tested the basis in its documented direction.**

**The chain** (`crates/strategy/src/cross_sectional/momentum.rs`, `selector.rs`):
1. `basis_warmed` holds `(symbol, −mean(basis))`, so a **high** score means a **low** basis (the basis-reversal convention: long the uncrowded name).
2. Ranks are assigned after a **descending** sort: `rank = i + 1` ⇒ **rank 1 = highest score = LOWEST basis = best**.
3. `residual = rank(basis) − rank(funding)`.
4. `top_k_long` sorts **descending** and takes the top k — the **highest** residual.
5. Highest residual ⇒ **large** `rank(basis)` ⇒ **worst** basis-reversal score ⇒ **HIGHEST basis**.

The doc block three lines above the function says the opposite: *"Long = highest residual (**low-basis** RELATIVE to its funding level)."* Same claim in `config.rs`. Low-basis-relative-to-funding is the **lowest** residual, not the highest.

**Worked example** (the story's own unit-test fixture): basis AA = −0.02, CC = +0.02; funding AA = +0.03, CC = −0.03.
- basis scores: AA = +0.02, CC = −0.02 ⇒ `rank(AA)=1`, `rank(CC)=2`
- funding scores: AA = −0.03, CC = +0.03 ⇒ `rank(CC)=1`, `rank(AA)=2`
- residual: AA = 1−2 = **−1**; CC = 2−1 = **+1** ⇒ `top_k_long` longs **CC**, whose basis is **+0.02, the highest**.

Under plain `ScoreSource::BasisReversal` on the identical inputs the arm longs **AA**. The residual arm is the basis-reversal direction **inverted on the basis axis**.

**Why no test caught it**: every residual test asserts *difference* or *inequality* — that the residual arm diverges from the raw basis arm — and none pins its **direction**. The nearest test bakes the confusion in: its own assertion message reads *"AA has high basis and low funding → residual must be negative"* while AA's basis is −0.02, the **lowest** in the fixture. A test whose prose contradicts its fixture cannot be a direction gate. (Compare the long-only basis arm, which has TWO literal-value sign guards precisely because R-BR.2 named the sign load-bearing; the residual arm inherited the requirement's importance but none of its guards.)

**What it does to the conclusion**: the recorded finding is *"residual arm: negative median Sharpe (2023 g0 p50 = −0.064) → basis carries no orthogonal alpha → derivatives-positioning domain CLOSED with finality."* If the arm ran inverted, a negative median is **consistent with** the basis carrying a real edge in the documented direction — the exact opposite reading. This does not establish that an edge exists (costs, the #73 10× over-accrual, the liquidation regime and ordinary noise all sit in the same number), but it does mean **the negative result cannot be read as "no orthogonal alpha" at all.** The honest status is: unknown, pending a correctly-signed re-run.

**Fix (1-25)**: decide the intended direction, make the code match the doc (or the doc match a deliberately-chosen code), and add a **literal-value direction gate** in the shape of the long-only arm's sign guards — an inequality test cannot hold a sign.
**Moral**: when a requirement is declared load-bearing for one arm, the declaration does not travel to the arm that derives from it. A derived signal needs its own direction gate, asserting a *value*, not a *difference* — "differs from the raw arm" is satisfied just as well by the inverse as by the intended construction.

### `#77` — A snapshot baseline regenerated in the same commit as the code it depicts cannot witness that code: it ratifies whatever the code produced
**Status**: the instance was FIXED 2026-06-11 (`3f9fd63`); the CLASS is disclosed here 2026-08-11 by the story-2-15 review, because nothing prevents its recurrence.
**Discovery**: found by two review layers independently while auditing story 2-15; both commit messages quoted below were verified verbatim by the orchestrator.

**The instance.** Story 2-15 wired the cockpit's Live KPI strip. `EquitySeries::max_drawdown_pct` is a **fraction** (0.40 = 40%); `BacktestMetrics.*_pct` are **percentage points**, and the formatter appends `%` verbatim. The wiring assigned fraction → percent field with no scaling, so a +10% session rendered **"0.10%"** and a 25% drawdown rendered **"−0.25%"** — money displayed 100× too small.

**Why four separate gates all went green on it:**
- two unit tests asserted the values the implementation produced (`dec!(0.10)`, `dec!(0.25)`) — i.e. they asserted the bug;
- two snapshot baselines were **regenerated in the same commit** and committed containing `card Total return: 0.10%` / `card Max DD: 0.00%`.

The fixer's own commit message states it (`3f9fd63`, verbatim): *"the wiring test had **encoded the bug as fact** (0.10) — corrected to 10/-10/25 + added `live_kpi_units_render_percent_not_fraction` pinning the rendered card text."*

**The class, stated generally**: a baseline or golden file regenerated from the code under test has **no independent authority over that code**. It cannot disagree with it. Committing it converts whatever the code currently does into the contract — so the next reviewer sees a green snapshot gate and a matching unit test, and both are circular. The defect was ultimately caught by the operator *looking at the screen*, which is the one oracle that was not derived from the implementation.

**This is why AD-10 says what it says.** "A passing proxy is not proof the screen draws" is usually read as "unit tests are weaker than pixels". The sharper reading is about **authority**: a test is only a gate if its expected value comes from somewhere the implementation cannot reach. A regenerated baseline, a test written by reading the implementation, and a text mirror that re-implements the view all fail that test regardless of how many of them there are.

**Moral**: when a change alters a number a human will read, the assertion must be an **independently-derived literal** — computed by hand, taken from the spec, or read off the requirement — never a regenerated baseline and never a value copied from the implementation's output. Regenerating a baseline is how you *record* a change; it is never how you *verify* one. Corollary for reviews: ask of every green baseline, "was this file regenerated in the commit it is guarding?" If yes, it is documentation, not a gate.

**RIDER (2026-08-12, story-2-18 review) — the harness removes the last friction: the visual gate MANUFACTURES its own expected value when the expectation is absent.** `crates/ui/tests/fixtures/visual_diff.rs`:

```rust
if !baseline.exists() {
    // First-run: persist the baseline so subsequent runs have something to compare.
    // Operator reviews the PNG before committing (H2 falsifier in feature.md).
    actual.save(baseline)?;
    return Ok(());
}
```

Delete a baseline PNG and the test **writes it and returns green**. `visual_snapshots.rs` documents that as the sanctioned accept-a-change workflow ("delete the baseline + rerun — helper auto-rewrites"). So the only thing standing between a silent visual regression and a green suite is a human remembering to open the PNG — a step no gate enforces and no artifact records.

Two consequences of record:
1. **A mass regeneration is a one-command operation with no per-file evidence.** Story 2-18's 56-file re-baseline was, on inspection, *rigorous* — orchestrator-verified pixel-by-pixel across all 56: content area byte-identical, the rest a pure one-nav-row translation. But the harness gave it no help; the rigor lived entirely in a dev-note nobody was required to write, and **nine subsequent re-baselines in this repo imitated none of it**.
2. **It is the live exposure for the pending ~62-file font-drift re-baseline** (story 6-9). That regeneration will pass green by construction whether or not the screens are correct.

**Rider moral**: a gate that supplies its own expected value when the expectation is missing is not a gate — it is a recorder with an assertion-shaped API. Absence of a baseline must FAIL loudly and require an explicit, recorded accept step; "first run writes it" is the same authority failure as "regenerate to make it pass", just earlier in the lifecycle. Relatedly: this is the vacuity class (#66) reaching the *harness* rather than a test — the missing expectation is auto-satisfied, so nothing can go red.

### `#78` — "Graceful degradation" that keeps a probe arm in the ranked field under its real label while running a completely different experiment — and names the substitute wrongly
**Status**: OPEN (disclosed 2026-08-12 by the story-3-15 review; the class has already propagated to a second arm). Anchor-impacting: NO.
**Severity**: product-honesty. It does not corrupt a number — it presents an experiment that never ran as one that did, on the screen a retail operator reads.

**The instance.** The DVOL implied-vol probe arm (`v0.dvol_regime`) is dropped from the bake-off field **only** when the coin is outside {BTCUSDT, ETHUSDT}. When the DVOL corpus is missing or its revision SHA mismatches, `resolve_dvol_override` returns `None`, the arm is dispatched anyway, and `cfg.dvol_override.clone().unwrap_or_default()` hands the engine an **empty series** — so the arm runs permanent warm-up: zero trades, 0% return, flat equity. **The DVOL parquets are gitignored** (only `REVISION.toml` is tracked), so this is the state on **every fresh clone, every CI box, every machine that has not run the fetcher**.

Meanwhile the leaderboard renders that row as *"Implied-vol regime (hold when DVOL < 30-day median)"*. A user reads "we tested the options/implied-vol channel on your coin". Nothing was tested.

**The naming defect that hides it.** Five places in the code call the degenerate fallback a **"buy-and-hold proxy"**. It is not: per bug-log-adjacent finding F1 of the same review, the warm-up path never emits a Buy (the arm initialises `weight: 1, is_long: false` and signals only on *transitions*, so `(1,1,false)` falls through to `Hold` forever). The fallback is a **100%-cash** arm. Two defects that are each survivable alone compose into a false disclosure: one makes the arm sit in cash, the other tells every future reader it is sitting in the coin.

**It propagated by citation, which is what makes it a class.** Story 3-16's macro-regime arm reproduces the pattern and says so verbatim: *"The arm still appears in the ranked field but runs warm-up-only (= buy-and-hold proxy, **identical to the `v0.dvol_regime` graceful-degradation precedent**)."* A pattern that is cited as precedent is no longer an instance.

**Why it is not a crown risk** (verified at source, so the disclosure is not overstated): a flat curve yields `prob_sharpe_gt_1 = 0`, below the FRAGILE band, so the arm is never `is_eligible` and cannot be crowned. The harm is **presentational**, not a wrong recommendation. That is the correct scope for this entry — but presentational harm is exactly the harm that matters for a product whose entire value proposition is honesty.

**SECOND TRIGGER, more dangerous because the integrity gate PASSES — and it is LIVE on the product's headline path today.** The first trigger (corpus absent) at least fails a revision check. This one does not: the corpus is present, its SHA verifies, and the arm still degenerates.

The advisor's relative lookbacks are anchored to wall-clock now — `DateRange::Custom { start_ms, end_ms }` with `end_ms == NOW` (asserted in `leaderboard/runner.rs`). The DVOL corpus is a **frozen pinned parquet set**, `generated_at = 2026-07-09T22:49:37Z`. Orchestrator-verified on this machine on **2026-08-12**:

| lookback | span | DVOL rows in span | result |
|---|---|---|---|
| TwoWeeks | 2026-07-29 → 2026-08-12 | **zero** | empty series → 100%-cash arm, labelled "Implied-vol regime" |
| OneMonth | 2026-07-13 → 2026-08-12 | **zero** | same |
| ThreeMonths+ | … → 2026-08-12 | rows stop 2026-07-09 | last close **frozen ~34 days**; `as_of` is LOCF with no max-age, and the ring only advances when the value *changes*, so the median freezes and the regime decision is a constant for the tail |

No warning reaches the screen in any of these cases. The gap widens by one day per day: **every pinned-corpus channel joined to a NOW-anchored window becomes silently inert with the passage of time alone**, and the SHA pin — the very mechanism meant to guarantee integrity — reports everything healthy. A calendar rollover is a third variant: `files_for_span` enumerates `<SYM>/<YEAR>.parquet` for every year in the span, so on 2027-01-01 a missing `2027.parquet` yields `RevisionMismatch` → the same silent stub.

**The published headline number is the signature of the bug, not a measurement.** The commit and CHANGELOG advertise "the arm diverges from buy-and-hold ~48k/49k USDT on BTC/ETH". Buy-and-hold on BTC H1-2024 is ≈ +47.78% — from 100,000 that is ≈147,780 — and an arm sitting in cash is exactly 100,000. The advertised divergence **is** the cash-versus-buy-and-hold gap, arithmetic that reproduces to within rounding. The decisive datum is already printed by the story's own gate: if `trades == 0`, the "pre-registered valid null" measured the warm-up defect rather than the implied-vol channel.

**Fix**: degrade to **ABSENCE**, not to a substitute. If a probe's input channel is unavailable *or stale relative to the requested window*, the arm must be dropped from the field on the same code path that drops it for an unsupported symbol, and the leaderboard must say the arm did not run (the ADR already required "DVOL-regime arm available for BTC/ETH only" copy — never built). A pinned corpus joined to a NOW-anchored window additionally needs a **staleness bound**: assert coverage of the requested span, not merely that the file's SHA matches. Until then, correct the five "buy-and-hold proxy" comments, which are false as shipped.

**Moral**: graceful degradation must degrade to *nothing*, never to *something else wearing the original's name*. A probe that could not run has one honest rendering — "not run" — and a row that looks like a result is worse than an error, because an error gets investigated and a plausible row gets believed. Corollary for reviewers: whenever you see a fallback described as equivalent to some benign baseline, check that it actually *is* that baseline; the description is written at design time and the behaviour drifts.

### `#79` — €200 lot realism is INERT on the advisor path: `venue_filter` is configured into every bake-off arm and never reaches the engine. The "ADVISOR-PATH GATE" that names it asserts a constructor value and never calls production.
**Status**: **FIXED 2026-08-12**, same session as the disclosure. 13 arms now thread `cfg.latency_slippage_sim` through `run_scenario` (14 total); the engine is built as `PaperEngine::new(..).with_venue_filter_mode(input.latency_slippage_sim.venue_filter)` at all three construction sites — the two in `sma_composed_run` plus the inline engine the 8 `v0.8.vote.*` arms build, which was outside the original diagnosis and would have left a third of the ranked field inert. The mis-named gate is re-pointed: five new tests call the production `run_scenario` with the ScenarioConfig `bakeoff/mod.rs` actually builds, and each asserts three independent witnesses — **traded** (non-empty fills, so it cannot pass by silently skipping), **mechanism** (every advisor fill is an exact multiple of `step_size` while the plain path has at least one that is not), and **effect** (terminal equities differ). The old constructor test is kept but stripped of its "ADVISOR-PATH GATE" claim and re-documented as proving nothing about the advisor on its own.
**Mutation-proven, both links:** reverting the *apply* step fails 3 tests with `€200 lot realism is INERT in production — advisor-path fills NOT multiples of step_size=1: 10/10`; reverting the *threading* for a single arm fails only the multi-arm sweep while the single-arm test stays green — which is exactly why the sweep exists. Blast radius measured on the real 2024 corpus at €200: typical arms move **< 0.1%** of terminal equity, sign varying by design (rounding down deploys less capital, which helps in down-legs and hurts in up-legs). Anchors `119 / 119` before and after; no anchored path calls `run_scenario`, and every non-advisor caller passes `venue_filter: None`.
**Deliberately NOT changed, and why:** nine other scenario runners (`momentum.rs`, `pairs.rs`, the TCN/PatchTST overlays, `regime_dispatcher.rs`, `garch_vol_target_overlay.rs`, `montecarlo.rs`, `threshold_sweep.rs`) build their engine the same way. They are frozen research/CLI lanes whose callers pass `None`, so the fix is provably inert there — pure anchor risk for no product gain. **Latent repeat**: after the threading fix, `v1.5a.pairs` and `v2.5.tcn*` now *receive* the config and still would not apply it, so adding either to the bake-off registry re-opens #79 silently.
*(Original diagnosis retained below.)*
**Was**: OPEN (disclosed 2026-08-12 by the story-3-15 review; **not that story's defect** — found through it). Affects **all ~14 bake-off arms**, not one. Anchor-impacting: **no** (bake-off/sweep paths run `write_report=false`; the plain `Default` stays `None`, which is what the anchored CLI lanes depend on per ADR-0087 §D6).
**Severity**: this is the product's headline promise. The Honest Advisor exists to hand a retail operator a **tradeable** plan on a **€200** budget; lot-size and min-notional filtering is what makes a recommendation executable rather than notional. PRD §13 Q5 decided "lot realism ON for the advisor path" and it shipped 2026-08-04 (`de571de`). It has never executed.

**The chain, every link orchestrator-verified at source:**
1. `bakeoff/mod.rs` (and 6 sites in `bakeoff/sweep.rs`) set `ScenarioConfig.latency_slippage_sim = LatencySlippageSimConfig::advisor_default()`.
2. `advisor_default()` does set `venue_filter: Some(VenueFilterMode::LotSizeAndMinNotional)` — verified in `cli_types.rs`. So far so good.
3. **`run_scenario` never threads it.** Of ~15 arms only `v1.momentum` passes `cfg.latency_slippage_sim` through; `v0.dvol_regime` and the 13 other `sma_composed_run`-shaped arms hardcode `LatencySlippageSimConfig::default()` → `venue_filter: None`.
4. **And it would not matter if they did.** `run_with_strategy` and `run` build the engine as `PaperEngine::new(match_config, seed)`, where `MatchConfig` carries only `slippage_bps` / `taker_fee_bps` / `maker_fee_bps` / `fill_price_mode` — **there is no venue-filter field on the path at all**, and `PaperEngine::new` defaults it to `None`.
5. `grep -rn with_venue_filter_mode crates/` returns **only** the builder's own definition in `paper.rs`, `paper.rs`'s unit tests, and `lot_realism_divergence_end_to_end.rs` — **zero production call sites.**

**Why every gate stayed green.** The file named `lot_realism_divergence_end_to_end.rs` — whose own section header reads "ADVISOR-PATH GATE" — builds its **own** `PaperEngine`, calls `.with_venue_filter_mode(...)` on it directly, and asserts that `advisor_default().venue_filter == Some(...)`. That is a **constructor-value assertion**. `grep -cE "run_bakeoff|run_scenario|sma_composed_run|run_with_strategy"` over that file returns **0**. It proves the constant is set; it proves nothing about the advisor.

This is the playbook § 6 rule at feature scale — *extracting a seam and testing the seam proves nothing about the binary* — the same lesson the 1-18 review learned when the burn-down's own 1-15 fix turned out to be inert in production. There it was one function; here it is a shipped product decision.

**What an operator gets today**: bake-off fills are computed with no lot-size rounding and no min-notional rejection, so the ranked plan can contain positions that cannot be placed at €200 on the venue it names — while the config, the ADR, the PRD row and a gate-named test all say realism is on. Note the honest half: the *advisor gate itself* (`bakeoff/bootstrap.rs`) resamples log-returns and is unaffected, so crowns and the ship-passive thesis do not move. The defect is in what the operator is told they can **do**, not in what beat what.

**Fix — two sites, both required, and the second is the one that has been missed twice**: (a) thread `cfg.latency_slippage_sim` through `run_scenario` for every arm, not just `v1.momentum`; (b) make `run`/`run_with_strategy` actually apply it — `PaperEngine::new(...).with_venue_filter_mode(cfg.latency_slippage_sim.venue_filter)`. Then re-point the advisor-path gate at `run_bakeoff`/`run_scenario` so it fails when either link is cut, and prove it by mutation. Keep the plain `Default` at `None` so the anchored CLI lanes stay byte-identical (ADR-0087 §D6).

**Moral**: a value is not "on" because a constructor sets it, a config carries it, an ADR ratifies it and a test named for it passes. It is on when a caller graph connects it to the thing that acts on it. For any feature flag, trace the path **from the config field to the line that reads it** — and if the only reader is a test, the feature does not exist. Corollary for gates: a test file whose name claims a production path must *call* that path; assert on the output of `run_*`, never on the value of a constructor.

### `#80` — Short legs bypass the matching engine entirely: the ranked field compares long arms that pay slippage and lot-rounding against short arms that pay neither
**Status**: OPEN (found 2026-08-12 while fixing #79; disclosed, not fixed — the fix is a real execution-model change and wants its own scoped story). Anchor-impacting: **needs measurement** (see below).

**The mechanism.** In `sma_composed_run.rs` the Sell-when-flat and Buy-when-short branches call `short_exec::try_open_short` / `try_cover_short` and then `continue; // handled; skip the matching-engine path`, hand-synthesizing a `FillView`. `short_exec.rs` holds no engine reference.

**What those legs actually model — verified precisely, because the first framing overstated it:**

| execution effect | long legs (via `PaperEngine::step`) | short legs (via `short_exec`) |
|---|---|---|
| taker fee | ✅ | ✅ — `taker_fee_bps` is a parameter and is applied |
| slippage | ✅ | ❌ — `grep slippage crates/backtest/src/short_exec.rs` returns **nothing** |
| venue filter (lot size / min notional) | ✅ since #79 | ❌ |
| fill-price model | ✅ (`FillPriceMode`) | ❌ — takes a `mark` directly |

So it is *not* "no execution model" — the fee is charged. It is a **partial** model, and the missing pieces are the ones that cost money.

**Why that matters more than it looks: it is an asymmetry inside a ranked comparison.** The bake-off ranks all arms against each other and crowns one. Long arms are charged slippage and have their sizes rounded down to tradeable lots; short arms are charged neither. Every short-enabled arm therefore carries a systematic, unearned advantage in the very comparison that decides what the operator is shown. Witnessed empirically during the #79 fix: `v0.sma_cross_ls` emits 194 fills on the gate corpus, of which **20 remain un-rounded on the advisor path** — precisely the short legs, visible because the long legs are now rounded and they are not.

**Scope check before anyone panics.** This does not touch the ship-passive thesis: the advisor gate (`bakeoff/bootstrap.rs`) resamples log-returns and the crowned arm across the corpus is the passive benchmark, not a short arm. The harm is that a short arm's ranking is flattered relative to its long siblings — a comparability defect, not a wrong headline.

**ANCHOR IMPACT — MEASURED 2026-08-14 (the entry previously recorded this as unmeasured; it is now resolved): NO.** The blast radius is confined to the advisor/bake-off lane. Verified by enumeration:

| lane | writes anchored bodies? | calls `short_exec`? |
|---|---|---|
| `scenarios/sma_composed_run.rs` (advisor / bake-off) | **no** (`write_report=false`) | **yes** — the bypass lives here |
| `scenarios/montecarlo.rs` (the anchored θ-surfaces, incl. the MN family with deliberate short traffic) | yes | **NO — zero calls.** Its three `short_exec` mentions are all doc-comments and one assertion-message string; its shorts go through `Order::new` + `engine.step` (10 call sites) |
| `bin/threshold_sweep.rs` | yes | **no** — zero references |

So the anchored surfaces do **not** run their short legs through the bypassed path, and fixing this cannot move an anchored number. **The blocker recorded here is cleared: #80 is safely fixable**, and the fix does not need to wait on a re-lock.

**Fix direction**: route the short legs through `PaperEngine::step` like every other order, or — if the engine genuinely cannot represent a short open/cover — give `short_exec` the same `MatchConfig` and venue filter and prove parity with a test that runs the same strategy long-only and short-enabled over a symmetric fixture and asserts the friction per unit notional matches.

**Moral**: a `continue` that skips the shared path is a fork in the execution model, and forks drift. The long path has since gained a solvency guard, a side-blind cap, slippage and a venue filter; the short path inherited none of them, and nothing compares the two. When a ranked comparison spans two execution paths, the ranking measures the paths as much as the strategies — so any `// skip the matching-engine path` deserves a test that pins the two paths to the same friction.

### `#81` — The macro-regime arm's loader is NEVER COMPILED. `v0.macro_riskon` has run 100% cash in every build of the product since it shipped, and no amount of data can fix it.
**Status**: OPEN (disclosed 2026-08-14 by the story-3-16 review). Anchor-impacting: **no** (every macro path runs `write_report = false`). **The most severe product defect of the burn-down**: not a distorted number, but a ranked strategy arm that has never once executed its own logic while being presented to the operator as one of the strategies tried.

**The chain, every link orchestrator-verified at source:**
1. `crates/backtest/src/macro_regime.rs` opens with `#![cfg(feature = "yahoo")]` — the entire loader module is gated on **`backtest`'s** `yahoo` feature.
2. `crates/backtest/Cargo.toml` declares `yahoo = ["data/yahoo"]` — the feature exists — but the file has **no `default = [...]` line at all**, so `default = []` and `yahoo` is off unless someone asks for it.
3. `grep -rn 'backtest/yahoo' --include=Cargo.toml .` → **zero hits.** Nothing in the workspace asks for it.
4. `crates/ui/Cargo.toml` declares `backtest = { path = "../backtest" }` — **no `features = [...]`** — so `backtest` is built with default features, i.e. without `yahoo`.
5. The near-miss that makes it invisible: `ui` *does* have its own `yahoo` feature, `yahoo = ["dep:data", "data/yahoo", "data/yahoo-online"]`. That enables **`data`'s** yahoo feature, not `backtest`'s. **Cargo features are per-crate and are not unified across crates** — so the workspace looks like it enables yahoo everywhere, and does not enable it where it matters.

Consequently `run_bakeoff` always takes the `#[cfg(not(feature = "yahoo"))]` branch, `preloaded_macro_series` is `None`, the engine arm builds an **empty** `PitSeries`, `as_of_value` returns `None` for every bar, the regime never turns on, and the arm holds **cash for the entire window** — while the leaderboard renders it as *"Macro regime (hold when SPX up, DXY down, rates calm)"*.

**Why this is strictly worse than #78.** #78's DVOL instance is a *data* problem: the arm degenerates when its corpus is missing or stale, and fetching the corpus fixes it. This is a *build-configuration* problem: the machine used for this review **has** the full macro corpus (2021-01 → 2026-06, all three tickers) and the arm is still inert, because the code that would read it was never compiled in. There is no runtime state in which this arm works.

**What it does to the product's central claim.** The advisor's credibility rests on "we tried N strategies honestly and none beat holding". The macro arm is counted in that field. So the count is inflated by an arm that never ran, and a channel the project reports as *tested and null* was never tested at all. The null may still be true — but it is unevidenced, and the record says otherwise.

**Not a crown risk** (verified, so the disclosure is not overstated): a flat curve yields `prob_sharpe_gt_1 = 0`, below the FRAGILE band, so the arm is never `is_eligible` and cannot be crowned. As with #78, the harm is presentational — which is the harm that counts for an honesty-first product.

**The two "graceful degradations" of this one arm are OPPOSITE, and the code says otherwise.** `crates/agent/src/runtime.rs` registers `AlwaysLongStrategy` for `v0.macro_riskon` in the forward paper loop, justified by this comment (verbatim): *"the arm degrades to buy-and-hold **exactly as `run_macro_gated_buyhold_path` does with an empty regime series**"*. That equivalence is false in both directions — orchestrator-verified: the bake-off path with an empty series holds **100% cash** (`prev_on` starts `false`, the flat→ON branch never fires, `coin_qty` stays 0), while `AlwaysLongStrategy` holds **100% coin**. So:

| context | what runs under the label *"Macro regime (hold when SPX up, DXY down, rates calm)"* |
|---|---|
| bake-off — which **ranks** the arm | 100% **cash** |
| forward paper run — which **executes** it | 100% **long**, ignoring `^GSPC`/`DX-Y.NYB`/`^TNX` entirely |

Whichever number the operator sees comes from a strategy that is not the named one, and the two contexts disagree about *which* wrong strategy to substitute. The comment states the substitution "prevents the `bail!` and lets the forward plan emit an honest 'BuyAndHold' description" — so it was deliberate, and rested on a false equivalence nobody re-derived. This also defeats the F5b anti-fake gate in the same file, which exists precisely to refuse a silent proxy fallback: the gate bails for an *unknown* arm and waves through a *known* arm wearing a substitute.

**The contamination reached the multi-corpus evidence.** `crates/backtest/tests/p2_verdict_rerun.rs` hard-codes `macro_riskon: true` for the 2021-22 corpus with the comment *"macro is symbol-independent … so it applies to all 10"* — but its window starts 2021-01-01, the 100-day warm-up reaches back to 2020-09, and the corpus has no `2020/` directory → `CacheMiss` → `None` → the macro arm ran **100% cash across all ten symbols of the 2021-22 bear regime and was then printed as an evaluated candidate** in the table behind the era-qualified thesis. The same harness *does* retain-filter the DVOL arm out on unsupported corpora and steps straight over its macro neighbour in the same function.

**A second defect waits behind it**, invisible while #81 holds because the code never runs: the three macro legs are keyed at `open + 24h` and their close instants are **disjoint** (`DX-Y.NYB` 05:00Z, `^TNX` 13:20Z, `^GSPC` 14:30Z), so the union loop emits ~**3 records per trading day** and only **1 of 3** is computed on same-day closes for all three legs — the other two mix vintages (e.g. SPX from D-2 against DXY from D-1). The pre-registered "3-AND rule at the daily close" is therefore evaluated on aligned inputs one time in three, and a day on which two legs flip produces two round-trips where the rule intends one. This is bug-log #73's loop-scope shape exactly: the rule fires per *ticker-close*, not per *macro day*. It must be fixed before the arm's output can be trusted, and it means fixing #81 alone would produce a *working but wrong* arm.

**HONESTY HALF FIXED 2026-08-14; the capability half is deliberately NOT flipped and is BLOCKED — see the sequencing note.**

What landed: the arm is now **dropped to ABSENCE** whenever its regime series is unavailable, mirroring the guard the DVOL sibling already had in the same function, so the leaderboard no longer shows a strategy row that never ran. The design point worth keeping: the field still *declares* the arm and the loop drops it, routed through a new single predicate `arm_runs_in_this_build(id)` that both the loop and the cockpit's arm-count read — deliberately **not** by making the field return empty, which would have made the new guard dead code in the shipped build and therefore unprovable. RED-proven: disabling the guard reproduces the shipped defect verbatim, printing a ranked `v0.macro_riskon` row with no regime series. The advisor field now honestly reports **19** arms, and 20 only if the capability is enabled. Also fixed: the forward loop now `bail!`s instead of substituting `AlwaysLongStrategy` (the test that asserted `Ok` was asserting the defect, and was inverted); the 3-AND rule was extracted to a production seam the loader calls so the tests finally bind it; S3 became a real causality falsifier via prefix-invariance; a span-coverage bound was added (weekend/holiday LOCF explicitly preserved — that part was always correct); and the tautological T-CAL tests now call the production functions, binding `expected_bars_for_range`, which previously had **zero** test call sites.

**⚠ THE DEFECT HAS A TWIN — `backtest/realdata`, found 2026-08-15 by generalizing this entry into a detector.** #81 was written as if `yahoo` were a one-off. It is not. The workspace has seven `cfg(feature)`-gated capabilities; testing each for "is it enabled by anything that ships?" finds **two** with the identical signature:

| feature | enabled by another crate? | in its own `default`? |
|---|---|---|
| `backtest/yahoo` | **no** | **no** | ← #81 as originally written |
| `backtest/realdata` | **no** | **no** | ← **the twin** |
| `data/yahoo` | yes (6) | no | |
| `strategy/forecast` | yes (3) | no | |
| `forecast/candle` | yes (1) | no | |
| `agent/in_process_cron` | yes (1) | no | |
| `ui/live` | — | **yes** | |

`backtest/realdata` gates `dvol_data`, `basis_data`, `funding_data`, the real-data scenarios, **and `resolve_dvol_override`** — whose `#[cfg(not(feature = "realdata"))]` variant returns `None` unconditionally. Verified end-to-end: `ui` declares `backtest = { path = "../backtest" }` with no `features`, `ui`'s entire `[features]` section mentions `backtest` **zero times**, `backtest` has no `default` stanza, and the documented run commands (`cargo run -p ui --release --bin cockpit_live --features live`, `cargo run -p ui --bin cockpit`) pass nothing that reaches it. **So in the operator's actual build, `backtest` compiles with NO features and the DVOL arm cannot load its corpus either — for exactly the same reason as the macro arm, not for the corpus-absence reason bug-log #78 describes.**

**This corrects #78's framing for the DVOL instance**: that entry says the arm degenerates "when the DVOL corpus is missing or its revision SHA mismatches", implying a data problem fixable by fetching. In the shipped cockpit it is unconditional — the loader is not compiled, so fetching cannot help. (#78's *class* stands, and its DVOL drop-to-ABSENCE guard already landed, so the arm is correctly absent today rather than ranked. The mis-description is what needed fixing.)

Note the `required-features` trap that hides both: `backtest/Cargo.toml` puts `required-features = ["realdata"]` on two of its **bins**. That gates those binaries, and reads like the feature is "used" — but it propagates nothing to library consumers, so the cockpit is unaffected by it.

**⚠ SEQUENCING — do not enable the feature yet.** The one-line change that would make the arm capable is `crates/ui/Cargo.toml`: `backtest = { path = "../backtest" }` → `backtest = { path = "../backtest", features = ["yahoo"] }`. **It must not be flipped before the emission-cadence defect below is fixed**, because it would produce a *working-but-wrong* arm rather than an inert one — the union loop emits up to 3 records per trading day with legs of mixed vintage, so the pre-registered "3-AND rule at the daily close" is evaluated on aligned inputs roughly one time in three, and a day on which two legs flip yields two round-trips where the rule intends one. The recorded "6 regime flips" is itself a cadence-inflated count. An inert arm that is honestly absent is a better state than a live arm computing the wrong rule. (Note also that even with the feature on, the default NOW-anchored lookbacks would still yield absence — the corpus ends 2026-06 — but now **loudly**, with the gap named, which is what the honesty fix buys.)

**Cost clause (pre-registration departure) — documented, economics untouched, operator's call.** Two options, both anchor-neutral: **(A)** charge the pre-registered taker fee in place (~10 lines; the recorded −0.39% becomes roughly −0.63%, Sharpe slightly more negative, no crown effect at any of these values) — this closes the pre-registration gap but leaves the asymmetry; **(B)** route the arm through `PaperEngine` so it also pays slippage and lot-rounding — **only (B) makes its friction comparable to the 18 siblings it is ranked against**, which is what the finding is actually about, and it is the same fix shape #80 wants for short legs.

**Moral**: a `#![cfg(feature = "…")]` on a whole module makes its absence *silent* — the crate compiles, the caller takes the `cfg(not(...))` branch, and the feature simply is not there. Combined with per-crate feature namespacing, a workspace can enable `x/yahoo` everywhere it is visible and still leave `y/yahoo` off. **Never let a product capability depend on a feature flag no test asserts is on.** For every `cfg(feature)`-gated capability, ask: which build actually ships it, and what fails if it does not?

### `#82` — The advisor's entire SHORT SLATE never shorts on real data. Five arms are ranked as long/short; four are long-only in practice, one never trades at all, and the flagship ratchets to ~11-16× leverage because the side-blind cap refuses its exits.
**Status**: **MECHANISM 1 FIXED — RE-MEASURED 2026-08-23** (by #71's resulting-exposure cap; gate added,
RED-proven). **Mechanism 2 still OPEN.** **Point 3 RETRACTED — it was a mis-framing, see below.**
(Found 2026-08-15 while fixing #80; the census that exposed it is now printed on every run so it
cannot hide again.)

---

#### RE-MEASUREMENT 2026-08-23 — same harness, same two real windows, same corpus

```bash
cargo test -p backtest --features realdata --test short_bakeoff_bear_bull -- --ignored --nocapture
```

| arm | window | fills then → now | buys/sells then → now | **short legs** then → now |
|---|---|---|---|---|
| `v0.sma_cross_ls` | bear | 182 → **254** | 181/1 → **126/128** | 0 → **128** |
| `v0.sma_cross_ls` | bull | 152 → **528** | 151/1 → **266/262** | 0 → **262** |
| `v0.macd_ls` | both | 66 / 189 (unchanged) | 33/33, 95/94 | 0 → **0** |
| `v0.rsi_ls` | both | 84 / 120 (unchanged) | 42/42, 60/60 | 0 → **0** |
| `v0.bbands_ls` | both | 134 / 206 (unchanged) | 67/67, 103/103 | 0 → **0** |

**Mechanism 1 (the leverage ratchet) is GONE.** `max_pos` fell from **28.2 / 39.2 units** to
**1.83 / 0.98**, `min_pos` is now **negative** (−1.81 / −0.94) where it was pinned at 0, and terminal
equity moved from **−9 235 / −14 146** to **positive on both windows** (bull: 88 887). The arm now
returns to flat, so `Sell-when-flat` fires and the short entry works — which is the whole causal
chain #82 described, run in reverse by fixing its root.

**Binding gate added, and RED-PROVEN by restoring the defect.**
`short_bakeoff_bear_bull.rs::assert_no_ratchet_and_shorts_taken` asserts, per window, that
`v0.sma_cross_ls` takes ≥ 1 short leg, reaches a negative position, and does not end at negative
equity. Reverting `Order::new`'s resulting-exposure check to the side-blind form fails BOTH windows
with `took 0 short legs` — reproducing #82's original measurement exactly. This is the assertion
#82's own moral said was missing: *"When an arm is labelled long/short, assert that it takes a short
— on real data."*

**Mechanism 2 (the alternation lock) is UNCHANGED and still open.** `macd_ls`, `rsi_ls` and
`bbands_ls` still alternate perfectly (33/33, 42/42, 67/67, 95/94, 60/60, 103/103) and still take
**zero** short legs. #82 predicted this would be independent of #71 and it was right. The gate
deliberately does NOT assert on these three: they need a signal-shape decision or honest
re-labelling, and asserting the current behaviour would encode a defect as a requirement.

**⚠ POINT 3 RETRACTED — `v0.always_short` taking zero fills is CORRECT BY CONSTRUCTION, not a
defect.** The arm dispatches to `bakeoff::buyhold::run_alwaysshort_path`, a closed-form
mark-to-market equity path — the exact structural twin of `run_buyhold_path`. It emits no `Fill`
because it constructs no `Order`, by design. The tell was in the same census table all along:
**`v0.buyhold` also reports `fills=0`**, and nobody thinks the benchmark failed to run. The harness's
own sanity gate proves the arm works — `initial=100 000 → final=156 210` on the −58 % bear window,
plus a direct `short_exec` cross-check at `+5 615`. Calling this "an arm which never trades" was the
same error #82 already self-corrected once in its scope note: **reasoning from a symptom without
tracing the implementation.** Twice in one entry.

 Anchor-impacting: **no** (advisor/bake-off lane, `write_report=false`). This is **#71's consequence on the product surface** — the research-lane defect was known; what is new is what it does to the ranked field the operator reads.

**Measured, orchestrator-verified, both real windows (2022-Q2 bear and H1-2024 bull):**

| arm | fills | buys / sells | short legs taken |
|---|---|---|---|
| `v0.sma_cross_ls` | 182 / 152 | **181 / 1** and **151 / 1** | **0** |
| `v0.macd_ls` | 66 / 189 | 33/33, 95/94 | **0** |
| `v0.rsi_ls` | 84 / 120 | 42/42, 60/60 | **0** |
| `v0.bbands_ls` | 134 / 206 | 67/67, 103/103 | **0** |
| `v0.always_short` | **0** | 0 / 0 | **0** |

**Two distinct mechanisms, both verified:**

1. **The leverage ratchet (`v0.sma_cross_ls`).** Buy-when-long adds 10% of equity on every bullish bar. Once the position passes `per_symbol_exposure_cap = 0.40`, every *closing* Sell is refused by `Order::new` — the **side-blind cap of bug-log #71** — while each individual *opening* Buy still passes, because each one is small relative to equity. So the position only grows: 181 buys against 1 sell, `max_pos` reaching 28.2 (bear) and 39.2 (bull) units on a 100k account, ≈11-16× leverage, ending at **negative equity** (−9,235 / −14,146 as measured during the #80 fix). The arm can never return to flat, so `Sell-when-flat` — the short *entry* — can never fire. **#71 does not merely leave a stale short open; on this lane it converts a long/short arm into an unbounded leveraged long.** Note the second-order trap: the cap check is guarded by `if current_equity > Decimal::ZERO`, so once equity goes negative the cap stops applying at all and the late Sell finally lands.
2. **The alternation lock (`macd_ls`, `rsi_ls`, `bbands_ls`).** These alternate buy/sell perfectly (33/33, 42/42, 67/67 …) — they never emit two Sells without an intervening Buy, so they never reach the flat state that `Sell-when-flat` requires. They are structurally incapable of shorting under this signal shape, independent of #71.
3. **`v0.always_short` takes zero fills on both windows** — an arm whose name is its entire specification, which never trades. That is bug-log **#78**'s class (an arm in the ranked field that did not run) on a fifth surface.

**⚠ SCOPE CORRECTED 2026-08-15 — this entry originally overstated the surface, and the correction is mine.** As first written it said the advisor "tells the operator it evaluated a long/short slate". **That is wrong.** `BakeoffConfig::default_short_field()` has **zero production callers** — verified: its only caller anywhere is `crates/backtest/tests/p2_verdict_rerun.rs`, an `#[ignore]`d harness. The cockpit's `advisor_field()` never includes these arms, so **the operator's leaderboard never runs the short slate at all**.

**What survives the correction, and it is still worth having.** Every measurement stands — five short-enabled arms, zero short legs across both real windows, `v0.sma_cross_ls` at 181 buys against 1 sell ratcheting to ~11-16x leverage and negative equity, `v0.always_short` at zero fills. What changes is *whose* record is damaged: not the operator's screen, but the **research lane** — specifically the P2 multi-corpus rerun, the harness that actually consumes `default_short_field()` and that feeds the evidence behind the era-qualified thesis. So a body of research evidence carries a "long/short" cohort that never shorted, alongside (per #81) a macro arm the same harness counted as "evaluated" while it ran cash. The mislabelling is real; the audience is the thesis record, not the retail operator.

**The lesson, recorded because it is the same failure I spent this burn-down naming in others:** I measured the arms correctly and then asserted a *reachability* claim — "the advisor evaluates these" — without tracing the caller graph. That is exactly the declared-not-executed error, committed while documenting it. Verify the surface, not only the symptom.

**Why it stayed invisible**: nothing counted short legs. The KPI columns show fills and trades, not sides; the exposure-cap rejection is silent (bug-log #71's own moral); and no test asserted that a short-enabled arm ever took a short. The `[SHORT-CENSUS]` line added during the #80 fix is the fix for *that* — it prints `short_legs=0` with an explicit warning marker on every run.

**Fix direction** (none taken — this needs a decision, not a patch): the cap must stop refusing position-*reducing* orders (that is #71's fix, and it is the load-bearing one here); the alternating arms need either a signal shape that can reach flat or honest re-labelling as long-only; and `v0.always_short` needs to either trade or be dropped from the field per #81's drop-to-ABSENCE precedent. **Do not simply raise the cap** — that would let the ratchet run further, not fix it.

**Moral**: a capability in a strategy's *name* is a claim, and nothing was checking it. When an arm is labelled long/short, assert that it takes a short — on real data, not a fixture built to make it. The census that found this is four lines; the absence of those four lines let five arms misrepresent themselves for the product's entire life.

### `#83` — The FROZEN robustness gate FAILS OPEN: a gate that errors is recorded as "skipped", and "skipped" is crown-eligible. Failure and deliberate-non-execution share one permissive flag.
**Status**: OPEN (found 2026-08-15 by the first audit of the frozen gate itself — 664 production lines that decide every verdict this project publishes, and which the 14-story burn-down never opened because AD-1 freezes them). Anchor-impacting: **no** (a fix changes no existing byte unless a candidate is currently hitting the failure path — see reachability below). **The gate is byte-frozen under AD-1, so this is a disclosure and a decision request, not a patch.**

**The chain, verified at source:**
1. `bakeoff/bootstrap.rs::compute_robustness_distribution` returns `Option`. It returns `None` on **three** conditions: `equity_decimals.len() < 2`; empty log-returns; and `DistributionSummary::from_path_metrics(...)` returning `Err` — which is the **NaN / non-finite guard**, a deliberate, well-reasoned check (ADR-0051 D2, extended by review 1-14 to ±∞) that correctly refuses to sort a metric vector containing NaN.
2. On that error the code does the right thing at source and then throws it away: `tracing::warn!(...)` and `return None`. Per this session's repeated finding, such a warn reaches no screen, no counter and no test.
3. `compute_robustness_flag` maps `None => RobustnessFlag::Skipped`.
4. `robustness.rs` documents `Skipped` as *"Gate intentionally not run (e.g. robustness disabled for a fast bake-off). **Crown-eligible** (treated same as `Robust`/`Marginal` by the comparator)."*
5. `rank.rs::is_eligible` confirms it: `c.is_benchmark || c.robustness != Some(RobustnessFlag::Fragile)` — **`Fragile` is the only ineligible flag.** `Skipped` is eligible. So is a `robustness: None`.

**So the gate fails OPEN.** A candidate whose robustness assessment *blew up* is treated identically to one the operator *chose* not to assess, and both are eligible to be crowned as the recommendation. The one flag that would stop it, `Fragile`, is precisely the one the failing path cannot produce.

**Reachability — stated honestly, because it decides severity.** I have **not** proven a live case. The bootstrap resamples log-returns and rebuilds equity from them, so the resampled paths likely stay positive and the NaN route may be unreachable today; `equity_to_log_returns_f64` also guards `prev <= 0.0`. The `len < 2` route is more plausible: degenerate arms demonstrably exist in the ranked field (bug-log #78, #81, #82 — arms that never trade, never load, or never short). What is certain is the **design**: the failure mode is permissive, and nothing distinguishes the two meanings after the fact.

**Why it matters more than its current reachability.** This project's entire credibility rests on the frozen gate being the thing that cannot be argued with — CLAUDE.md's non-negotiable is "no shipping on a REGRESSION verdict", and AD-1 freezes these files precisely so the bar cannot drift. A safety gate whose error path *widens* eligibility inverts that guarantee at exactly the moment it is most needed. Every other defect in this burn-down was a claim that outran the code; this is the guardrail itself defaulting to permit.

**Fix direction (needs an AD-1 decision, deliberately not taken):** give failure its own flag — e.g. `Errored`, ineligible and loudly reported — so that "not run" and "ran and failed" stop sharing a verdict; and make `is_eligible` allow-list the acceptable flags rather than deny-list the single bad one, so a future variant is ineligible by default. Note the second half is the more durable change: an allow-list cannot be widened by adding an enum variant, a deny-list silently is.

**Moral**: a gate's failure mode is part of its contract. Ask of every guardrail: *when this cannot decide, does it permit or deny?* Deny-listing one bad state means every state you have not thought of — including "the check crashed" — is a pass. And a `warn!` on the way out is not a report; it is a decision to continue, written in a tone that sounds like caution.

### `#85` — Two account-level loss stops are configured, defaulted, documented as kill-switch triggers, and read by nothing. The declared risk floor does not exist.
**Status**: **OPEN — interim honesty fix APPLIED 2026-08-15; the wire-or-delete decision is still the operator's.** (Found by the reachability sweep; every link orchestrator-verified, with one correction below.) Anchor-impacting: **no**.

> **Correction to the evidence above.** The soak runbook quoted below lives at `docs/archive/pre-bmad-spec/v1/paper-soak-longevity/runbook-realtime-soak.md:294-295` — that is **FROZEN history**, not a live operational runbook. The text says what the entry says it says, but an operator following *current* documentation never reads it. That weakens the "documented as kill-switch triggers" leg specifically; the config files and the `RiskConfig` defaults, which are live, are unaffected. The archived file was deliberately **not** edited (`docs/archive/` is frozen by CLAUDE.md).

> **Interim applied — the config no longer implies a guarantee.** Per this entry's own recommendation: both fields in `crates/agent/src/config.rs` now carry ⚠️ **NOT ENFORCED** doc comments naming the bug and stating that no read site exists, and `config/agent.toml` + `config/agent.toml.soak-fast` each carry a header comment saying the same in the operator's own file. `per_symbol_exposure_cap` is documented alongside as **ENFORCED**, so the contrast is visible at a glance. Verified: clippy `-D warnings` exit 0, fmt clean, 29 agent config tests pass (the files still parse). **Zero behavioural change.**

> **Sizing evidence for the real fix — wiring is modest, not a feature build.** The infrastructure is already complete: `KillSwitch::trip(reason)` is a public API (`kill_switch.rs:274`), it broadcasts `AgentMode::Halted`, it has an audit path, and the forward loop already computes `cur_equity` on every bar (`runtime.rs:2615`). What is missing is two `HaltReason` variants and the comparisons. **The enum is the tell**: `HaltReason` = {HaltFile, HeartbeatTimeout, LedgerImbalance, ClockSkew, ManualOperator, Test} — six variants, and neither loss stop among them. A configured stop with no `HaltReason` cannot trip anything, which is the whole defect in one line.

> **Why the wiring was not applied.** It changes runtime behaviour — runs that previously continued would begin halting — and that is an operator decision, not a cleanup. Paper/sim only, so nothing is at risk in the meantime.

**The evidence, complete:**
- `crates/agent/src/config.rs` declares `RiskConfig { per_symbol_exposure_cap, daily_loss_stop_pct, max_drawdown_stop_pct, … }` and defaults the two stops to **−5.0** and **−15.0**.
- `config/agent.toml` sets `daily_loss_stop_pct = -5.0` and `max_drawdown_stop_pct = -15.0`. So does `config/agent.toml.soak-fast`. These are the operator's live configs, not samples.
- The soak runbook documents both as kill-switch trip conditions in as many words: *"`max_drawdown_stop_pct` exceeded (-15% by default) — equity fell > 15% below peak"* and *"`daily_loss_stop_pct` exceeded (-5% by default) — single-day loss > 5%"*.
- ADR-0010 refers to `max_drawdown_stop_pct` as "the portfolio-level floor".
- **Read sites in the entire workspace: ZERO.** `grep -rn '\.daily_loss_stop_pct\|\.max_drawdown_stop_pct' crates/` returns nothing. All 14 occurrences are the two declarations, the two defaults, and struct-literal writes in a dozen test files. The values are deserialized and then never consulted by anything.

**Scope, stated honestly so this is not over-read.** This project is **paper/sim only** — the operator removed the live-execution program, and no venue-write path exists (independently confirmed by the claims ledger). So no real money is exposed today and this is not an active financial risk. What it is: a **safety control that is declared, configured, documented and absent** — and the specific danger of that shape is that it looks present. An operator reading `config/agent.toml` has every reason to believe a −5% daily stop is armed. It is not. If live execution were ever restored, the absence would be silent and the config would still read as protection.

**Same class as #79** (`venue_filter` set into a config by a constructor and never reaching the engine), applied to a risk limit rather than an execution parameter — and note that #79's fix does *not* cover this: they are different config structs on different paths.

**Fix direction**: either wire both stops into the kill switch that the runbook says they trip, or delete them from `RiskConfig` and both config files and strike the runbook lines. **Do not leave them declared-but-inert**, which is the current state and the worst of the three. If wiring them is out of scope for now, the honest interim is to mark them clearly in the config as *not yet enforced*, so the file stops implying a guarantee.

**Moral**: a risk limit is not a value in a config struct; it is a comparison somewhere in a hot path. For every declared limit, find the line that acts on it — and if the config is the only place the number appears, the limit does not exist. The playbook's binding-limit probe asks "construct a scenario where this must bind — does it?"; this pair never even reaches a reader, which is the degenerate case of the same question.

### `#86` — AD-18's ADR gate is ONE-WAY: it checks that every decision file has a registry row, never that every row has a file. ADR-0079 is a row with no decision — cited by production source and by another ADR.
**Status**: **FIXED 2026-08-15** (found by the claims ledger, verified at source and resolved by the orchestrator). Anchor-impacting: **no**.

> **Resolved via option 1 — write the decision, then enable the converse check, in that order.** The entry originally withheld the fix because enabling the check would turn a green gate red and block commits. Doing the two in sequence removes that objection entirely: the gate never goes red because the gap is closed first.
> 1. **`0079-shared-vol-estimator.md` written.** A **reconstruction from primary sources only** — the registry row, the module's own doc-comment (which records the design decisions verbatim, including the operator-ratified D5 home ruling), ADR-0078, and `v2-architecture.md` §6.0 D5. The file says so in its own header and preserves the original 2026-06-30 decision date; nothing was invented. Corpus is now **87 files = 87 rows**.
>    It also records, marked as an audit note rather than silently reconciled, that consumption is narrower than the design anticipated: `drawdown_control_overlay` — a *declared* consumer — never wired up, and **3 of the 4 functions** (including the full Corsi-2009 HAR-RV) have **zero production callers**. The module doc used the future tense (*"will call these"*), so that is planned-not-arrived rather than false; it is recorded because it is the same declared-vs-executed shape as #81/#85/#89.
> 2. **Invariant (d) `decision-file-missing` added** to `scripts/adr_registry_check.py`. **RED-proven**: hiding `0079-*.md` makes the gate emit `(d) decision-file-missing … write 0079-<slug>.md, or delete the ADR-0079 row (numbers are never reused)`; restored → exit 0. Self-test 5 → 7 cases, with case 7 asserting (a) and (d) remain **independent** directions so a future "simplification" cannot collapse them back into one.
> 3. **A second defect, found inside the gate itself, fixed in the same pass.** `_check_invariants` (what `_run_pre_commit` calls) and `_check_invariants_raw` (what `--self-test` called) were near-identical copies — so **the self-test exercised the copy and never the production function**, and an invariant added or changed in one alone would have stayed green. That is bug-log #77's shape *inside the lint*. They are now one implementation, with a comment saying not to re-inline it.
> 4. The new check immediately flagged a latent inconsistency in the gate's **own** case-1 fixture (a registered id with no file) — invisible for as long as only direction (a) existed. Fixture corrected.

> Verified: `adr_registry_check.py` exit 0 · `--self-test` 7/7 OK · anchors 119/119 · spec-lint PASS + self-test PASS. The ADR README changelog and `updated:` marker were bumped in the same pass so invariant (b) is satisfied when this commits.

**The invariant.** CLAUDE.md / AD-18: *"Every non-trivial decision is a numbered ADR under `architecture/decisions/` **plus** its Registry row in the same commit — enforced by `scripts/adr_registry_check.py`. Numbers are never reused."* That is a **two-directional** requirement.

**The gate enforces one direction.** `scripts/adr_registry_check.py` builds `registered_ids` from the README's `## Registry` table and then checks, for each discovered ADR *file*, `if num not in registered_ids` → fail. There is no converse pass: no id in the registry is ever checked for a corresponding file. So a decision can be **announced without being made**.

**And one has been.** The corpus holds **86 ADR files against 87 registry rows**; `0079-*.md` does not exist. Row 0079 is nonetheless present, `accepted`, dated 2026-06-30, and unusually detailed — it names the module (`crates/strategy/src/vol_estimator.rs`), its four functions, three exported λ constants, an operator-ratified home decision ("`crates/strategy` — NOT `forecast`"), and an anchor-safety argument. A reader has every reason to believe the decision exists.

**It is load-bearing, which is what lifts this above bookkeeping:**
- `crates/strategy/src/vol_estimator.rs:1` — *"Shared multi-horizon σ̂ vol estimator (P1-5 / ADR-0079)."*
- `crates/strategy/src/lib.rs:49` — same citation.
- `ADR-0078` line 101 — **"Consumes: ADR-0079 (`vol_estimator`)."** An accepted decision declares a dependency on a decision record that was never written.

So shipped code and an accepted ADR both point at reasoning that does not exist. The *what* survives in the registry row; the *why*, the alternatives, and the consequences — the entire reason an ADR corpus exists — were never recorded. `adr_registry_check.py` exits 0 throughout.

**Why the fix is not applied here.** Adding the converse check is ~5 lines, but the gate runs in `.githooks/pre-commit` and in CI, so enabling it turns a currently-green gate **red immediately** and blocks commits until someone either writes `0079-*.md` or deletes the row. That is a real decision with a real cost and it is the operator's, not mine. The two honest resolutions:
1. **Write the missing decision** (preferred — the row's detail suggests the reasoning existed at the time and simply was not committed), then enable the converse check.
2. **Delete the row** and strike the three citations — only if the decision genuinely was never made, which the ADR-0078 dependency argues against.

Either way the converse check should land in the same pass, otherwise the next one is invisible too.

**Moral**: a bidirectional invariant needs a bidirectional gate. "Every A has a B" and "every B has an A" are different checks, and enforcing only the cheap direction leaves the expensive one to discipline — which is precisely the arrangement bug-log #66 was written to end. Count both sides: 86 files against 87 rows was visible from `ls | wc -l` the whole time.

### `#87` — A documented operator opt-in cannot work in any build, and its off-path is a single silent `let _ =`
**Status**: **HALF-FIXED 2026-08-15** (found by the 24-feature cfg audit; framing corrected, then the silence fixed, by the orchestrator). Anchor-impacting: **no**.

> **What was fixed:** the silence. `crates/agent/src/runtime.rs` — the `cfg(not(feature = "forecast-audit-tick"))` arm now checks `cfg.strategies.tcn_overlay_momentum.enabled` and, when the operator has actually set the documented flag, emits a `tracing::warn!` naming the flag, stating it has **no effect** in this build, and giving the rebuild command. It is the same shape the enabled arm already uses when it skips for a checkpoint- or config-load failure, and it fires only when the flag is set — no noise otherwise. Verified: `cargo clippy -p agent --all-targets -- -D warnings` exit 0 (the edited arm **is** the default build, so this compiles the change directly), `cargo fmt --check` clean, `cargo test -p agent --lib` 101 passed / 0 failed.

> **Not gated by a test, stated plainly:** the repo has no log-capture harness (`grep` for `tracing-test` / subscriber-capture across every `Cargo.toml` and `tests/` returns nothing), and adding a dependency to assert one log line would be disproportionate. The warn is verified to compile, not to fire.

> **What is NOT fixed — the operator's half.** The flag still cannot work in *any* build: nothing in the workspace enables `agent/forecast-audit-tick`. The fix converts a silent no-op into a **loud** no-op, which is strictly better and is the house style, but it is not the same as making the opt-in functional. That remains a decision: **wire the feature** into a build that ships, or **remove the documented flag** from `config.rs` so the manual stops promising it. Leaving it documented-but-unreachable is the state this entry was opened about.

`crates/agent/src/config.rs` declares `RuntimeConfig.strategies.tcn_overlay_momentum` and documents it as *"Opt-in via `[strategies.tcn_overlay_momentum] enabled = true`"*. `crates/agent/src/runtime.rs` reads that flag **inside** `#[cfg(feature = "forecast-audit-tick")]`, and the `cfg(not(...))` arm is literally `let _ = ledger;` — no `bail!`, no `warn!`, not one log line. Nothing in the workspace enables `agent/forecast-audit-tick` (it is declared in `agent/Cargo.toml` and chains to `strategy/forecast-audit-tick`, but no dependent ever requests it, and `ui` passes nothing through).

So an operator who sets the documented flag gets **silence**: the config parses, validates, has passing unit tests for the field, and the registration it controls was compiled out. This is bug-log #81's shape with a **config file** as the operator surface instead of a `--features` flag — and note the contrast that makes it a defect rather than a design choice: 10 of `backtest/realdata`'s 13 off-arms `bail!` with the rebuild command. A loud off-path is the house style; this one is mute.

**Framing corrected on verification.** The audit reported the stanza as present and `enabled = true` in `config/agent.toml`. It is not there — that file enables `sma_crossover` and two others, and the tcn settings live in a separate per-strategy file under a different key. So no shipped config currently trips this. The defect is the **documented opt-in that cannot work**, not a flag currently set and ignored; recorded at that severity.

**Fix**: give the off-arm a `bail!` or a startup `warn!` naming the required feature, matching the ten sites that already do. If the capability is not intended to ship, strike the opt-in from the config docs so it stops advertising a control that does not exist.

### `#88` — AD-10's entire rendered-pixel evidence base is macOS-only, so two of the three CI legs report PASS while executing zero pixel assertions
**Status**: OPEN (found 2026-08-15 by the 24-feature cfg audit; scope verified by the orchestrator). Anchor-impacting: **no**.

**Measured**: `32` test files carry `#![cfg(target_os = "macos")]`, and `31` of them contain screenshot/pixel assertions — `visual_snapshots.rs`, `leaderboard_scorecard_render.rs`, `crown_credibility_render.rs`, `benchmark_wins_render.rs`, the `_audit_group_*_render` set, and the rest. On the Linux and Windows legs those files compile to **zero tests**, and the legs go green having verified nothing on that surface.

**Why this is not simply ADR-0057.** That ADR deliberately made macOS the canonical box for **byte-exact** baselines, and it is right: glyph rasterization differs across OSes, so cross-OS byte comparison is meaningless. That reasoning covers the byte-compare baselines. It does **not** cover the *structural* harnesses — the ones that count ACCENT-hue pixels or assert a populated curve draws more ink than an empty one. Those compare a render against itself, not against a stored image, and would run anywhere. Two files already opt out of the macOS floor deliberately to hold a cross-OS line, which shows the distinction was understood and then not applied broadly.

**The consequence is a false signal, and it compounds the burn-down's findings.** AD-10 is the non-negotiable that says UI ships only on rendered-pixel proof — and the burn-down found three separate stories whose AD-10 evidence could not fail (#74's class, plus the 2-15 and 2-18 records). A 3-OS matrix reporting PASS on legs that execute no pixel assertion is the same illusion one layer up: the *breadth* of the matrix implies coverage the matrix does not have.

**Fix direction**: split the gate. Keep `#![cfg(target_os = "macos")]` on byte-comparison baselines (ADR-0057 stands), and remove it from the structural pixel-count harnesses so all three legs run them. Where a harness genuinely cannot run headless on Linux, say so in the file rather than gating it by platform — a skip that is visible is worth more than a green that is empty.

### `#89` — `PaperEngine`'s seed is provably inert: the RNG it constructs is never read, and the `#[allow(dead_code)]` that hides it is the confession
**Status**: **PARTLY FIXED 2026-08-15 — the predicted tautology was found, PROVEN, and replaced; the wire-or-delete call remains the operator's.** (Found by the cfg audit's `#[allow(dead_code)]` sample; verified at source by the orchestrator.) Anchor-impacting: **no** — see below, and that is *why* it went unnoticed.

> **The predicted testing consequence was real and already shipped.** This entry warned that any seed-varying test would be tautological. `paper::tests::t24_deterministic_across_runs` was exactly that: it built two engines with the **same** seed (42, 42) and asserted the fills matched — an assertion that holds for any seeds and for no RNG at all. **RED-proven 2026-08-15**: mutating the second seed to `999_999` left the test *passing*, so a test named `deterministic_across_runs` could not detect seed-dependence. (`paper.rs` restored byte-identical afterwards; `git diff` clean.)
> **Replaced with the invariant that is actually true and actually falsifiable**: the two engines now take **deliberately different** seeds and assert the fills are identical — i.e. fills are **seed-INDEPENDENT**, which is the real current property. It carries a note that going RED means someone wired `self.rng`, and that the anchor story must be updated in the same pass rather than the seeds re-pinned to match. The field itself now carries a ⚠️ INERT doc comment recording that the `#[allow(dead_code)]` is the compiler's finding, silenced.
> Verified: `cargo test -p backtest --lib` 225 passed / 0 failed · clippy `-D warnings` clean · fmt clean · **anchors 119/119** (paper.rs is on the fill path, so this was checked, not assumed).

> **Still the operator's call**: wire the RNG into the fill path it was built for, or delete the field and the `seed` parameter. Deletion touches one production caller (`agent/runtime.rs:2128`) plus several tests. The interim removes the misleading half — a test that implied seedability was verified — without pretending the parameter now does something.

`crates/backtest/src/paper.rs` declares `rng: ChaCha20Rng` on `PaperEngine`, seeds it in the constructor (`ChaCha20Rng::seed_from_u64(seed)`), and **never reads it** — `grep 'self\.rng'` over the file returns nothing. Sitting directly above the field is `#[allow(dead_code)]`. rustc *did* detect this; the annotation silenced it.

**What that means for every caller.** `PaperEngine::new(match_config, seed)` advertises a seeded, reproducible matching engine. It is reproducible — but by construction, not by seeding: nothing in the fill path is stochastic, so the seed cannot change any outcome. Every call site that threads a seed through to the engine is threading a value with no consumer.

**Why it is anchor-safe and why that is the trap.** The anchors are byte-identical precisely *because* nothing is random, so this defect can never break the gate — which is exactly why it survived 119 locked anchors and a 14-story review. A parameter that does nothing is invisible to a determinism gate; determinism is what it looks like.

**The testing consequence is the real one.** Any test that varies a fill seed and asserts a difference is asserting something unreachable; any test that varies a fill seed and asserts *identity* is a tautology dressed as a determinism proof — the shape bug-log #77 names, and the shape the 1-21 review already found in `run_path_k_short_zero_byte_identical_to_head`. (No such assertions exist today — `grep fill_seed … assert|differ|identical` is empty — so this is a latent trap rather than a live false gate. The FILL_SEED domain-separation rider already queued for story 1-25 should be read in this light: separating domains of a value nothing consumes buys nothing until the value is wired.)

**Fix direction**: either wire the RNG into the fill path it was built for (slippage jitter, partial-fill sampling — whatever the seed was meant to drive) and give it a test that varies the seed and observes a difference, or **delete the field, the parameter and the `#[allow(dead_code)]`**, and let the signature stop promising reproducibility it does not provide. Do not leave it seeded-and-unread, which is the current state and the one that misleads.

**Moral**: `#[allow(dead_code)]` in production is a claim that the compiler is wrong. It is occasionally true. Treat each one as an unreviewed assertion — this repo carries ~42, and sampling 20 of them found 3 stale and 2 marking genuine inertness defects. A suppression is a place someone decided not to answer a question.

### `#90` — forced liquidation is invisible to BOTH short/long friction gates: it covers at the raw mark and emits no `Fill`
**Status**: **OPTION 1 APPLIED 2026-08-15 — carve-out documented at source and GATED; options 2/3 remain the operator's.** (Found while closing #80's forward half; verified by the orchestrator.) Anchor-impacting: **no**.

> **What landed.** (a) `check_and_liquidate` now carries a ⚠️ doc block at its definition stating that it emits no `Fill`, that **both** parity gates are blind to it *by construction*, why it is symmetric rather than a repeat of #80, why engine-routing was deliberately deferred (slippage moves the cover price, which moves the equity that triggered the liquidation — a feedback loop, not a friction correction), and the instruction to update this entry *before* adding a caller.
> (b) A new gate, `crates/backtest/tests/liquidation_carve_out_census.rs`, locks the **caller set** — the only defence available, since no fill-tape gate can ever see this path. It scans every `crates/*/src/**/*.rs`, strips line comments so the new doc block is not miscounted, and fails if any file outside the three-entry allow-list calls the function. It also asserts the **converse**: every allow-list entry must still call it, so a stale entry cannot quietly shrink what the gate guards.
> **RED-proven**: planting a call in an unlisted file fails with `BUG-LOG #90 — the forced-liquidation carve-out has WIDENED`, names the file, and explicitly says *"Do NOT just add the file to ALLOWED"* — because widening the allow-list is exactly the silent widening this guards. Removed → green, no residue. Non-vacuity guard included: the scan asserts it walked >100 files, so a broken walker cannot pass as clean.

> **Still open**: options 2 (emit a zero-slippage liquidation `Fill` so the tape is complete and the gates exclude it *explicitly* rather than structurally) and 3 (route it through the engine — the only one that changes results, and the one that needs its own blast-radius measurement).

`short_exec::check_and_liquidate` (`crates/backtest/src/short_exec.rs:375`) force-covers a short at the **raw `mark`** with the taker fee only — no slippage, no engine — and returns **no `Fill`**. Its two production callers are `agent/runtime.rs:2616` and `scenarios/sma_composed_run.rs:804`: one on each side of the fork #80 closed.

**Why this is not a repeat of #80.** #80 was an *asymmetry* — short legs paid less than long legs on the same path. This is *symmetric*: both halves call it identically, and long positions have no analogous forced-exit path to be cheaper than. Nothing is being under-charged relative to a sibling.

**Why it still matters.** Both friction-parity gates — the `backtest` one from #80 and the `crates/agent` one added today — measure the **fill tape**. A path that emits no fill cannot appear on that tape, so neither gate can observe it, in either direction. The gates are sound about what they see; this is simply outside what they can see. That is the #74 channel-probe shape: the question is never only "does the assertion hold" but "**can the assertion's input reach this path at all**."

**Why it was left alone, and this is the right call.** `apply_engine_fill`'s own doc-comment records that the #80 ranking-side fix scoped forced liquidation out *on purpose* — a forced cover "is deliberately allowed to drive cash negative." Routing it through the engine would change **what** gets liquidated and **when**, not merely what it costs: slippage moves the cover price, which moves the equity that triggered the liquidation in the first place. That is a feedback loop, not a friction correction, and it wants its own decision rather than a quiet extension of #80's.

**Operator decision required** — three coherent options, in increasing cost:
1. **Document the carve-out** and add a gate asserting forced covers stay off the fill tape, so the exemption is deliberate and can't silently widen. Cheapest; makes the current state honest.
2. **Emit a zero-slippage `Fill`** marked as a liquidation, so the tape is complete and the parity gates can exclude it *explicitly* rather than structurally. Makes the blind spot visible without changing economics.
3. **Route it through the engine.** Most faithful, and the only option that changes results — it needs its own before/after blast-radius measurement because of the trigger feedback loop above.

**Moral**: closing a fork does not close a *family*. After unifying two paths onto one seam, enumerate every OTHER exit from the same state — here, forced liquidation was a third exit that neither unified path used. The caller census that proved `try_open_short`/`try_cover_short` are now production-dead is exactly what surfaced the sibling that isn't.

### `#91` — `spawn_lab_run`'s non-`live` arm reports a **successful run that did nothing**, where both its siblings return `Err` — written for a build configuration that lasted one day
**Status**: OPEN, **LOW** severity (found by the cfg audit as NEW-B; reachability resolved by the orchestrator 2026-08-15). Anchor-impacting: **no**. Recorded with a deliberate **downgrade** — see "What is NOT true."

`crates/ui/src/lab/runner.rs:1274-1291`, the `cfg(not(feature = "live"))` arm, returns
`Message::LabRunCompleted(**Ok**(RunSummary{ equity_series: [], fills: [], kpis: default, report_path: None }))` — the exact wire shape of a real run that produced nothing. Its two siblings answer the same case the opposite way:

| function | `cfg(not(live))` behaviour |
|---|---|
| `spawn_lab_run` | **`Ok(RunSummary{ empty })`** |
| `spawn_bakeoff` (`leaderboard/runner.rs:242`) | `Err(LEADERBOARD_RUN_NEEDS_LIVE)` |
| `spawn_training_run` (`lab/trainer.rs:363`) | `Err("training not supported in non-live fixture builds")` |

Two of three bail; one returns a plausible value. That is the discriminator this audit was built around, and it is the same severity axis that separated `backtest/candle` (harmless — all off-arms `bail!`) from `backtest/realdata` (**#81** — returns a bare `None`).

**What is NOT true — the honest downgrade.** The audit flagged a second, scarier instance: the same empty-`Ok` on a **runtime** branch (`rt_handle == None`, `:1297-1310`) *inside the shipped cockpit*, and explicitly recorded that it had not determined whether production could reach it. **It cannot.** `cockpit_live.rs:130` declares `rt_handle: tokio::runtime::Handle` — **not** an `Option` — built at `:435` from `agent_runtime.handle().clone()`, and the sole production caller (`:2264`) passes `Some(&self.rt_handle)` unconditionally. The `None` arm is unreachable from any shipping path. The alarming half of NEW-B is **false**, and is recorded here so it is not re-opened.

**Nor does any build compile the `cfg` arm today.** `ui`'s `default = ["live", "yahoo", "binance"]`, and `--features fixtures` *adds* to defaults rather than replacing them, so `cockpit`, `ui-gallery` and `cockpit_live` all build with `live` **on**. Confirmed against cargo's own resolver, not by reading manifests.

**Why it is still worth an entry — the dated part.** `git log -L` puts the stub's rationale ("Fixtures / no-`live` / no-runtime mode: immediately resolve… useful for the fixture cockpit") at **79fceb5, 2026-05-24**. `live` was promoted to a default feature at **78d8fd2, 2026-05-25** — **the next day**. The branch was written for a build configuration that existed for roughly twenty-four hours; its stated purpose died with it, and the comment has asserted that purpose for three months since. This is a **stale rationale**, not a live defect: the code is unreachable, and the comment explains why it *should* be reachable.

**Fix direction (cheap, zero present risk — nothing compiles it):** make the arm return `Err` like its two siblings, and delete the stale rationale. If the empty-`Ok` is instead wanted deliberately, say so in the comment and note that no current build takes the branch. Either is fine; the current state — an unreachable branch defended by a reason that expired — is the one that misleads the next reader.

**Moral**: an unreachable branch is not automatically harmless, and it is not automatically a defect either. What makes this one worth logging is that its **comment out-lived its build**. Date the rationale, not just the code: `git log -L` on the explaining comment versus the commit that changed the world it described turns "this looks odd" into "this expired on a known day."

### `#92` — the `--no-default-features` build that `Cargo.toml` documents as supported does not compile, and nothing has ever built it
**Status**: **OPEN — manifest honesty applied 2026-08-15; the three-way resolution is still the operator's.** (Found while attempting to compile-verify #91's fix.) Anchor-impacting: **no**. **Blocks #91.**

> **Interim applied.** `crates/ui/Cargo.toml`'s comment previously advertised `--no-default-features` as a supported opt-out. It now says the opposite in as many words: the configuration does not build, names the three offending files, records that nothing in CI/scripts/skills/README/runbooks builds it, and states that no shipping target needs it (`ui-gallery` and `cockpit` both build with defaults on). Zero risk — a comment; `cargo metadata` verified still clean.

> **Scope re-measured, and it is smaller than the raw count suggests.** The "21 references" is misleading: **most of `state.rs`'s 16 `agent::` mentions are doc comments.** The actual compiled surface is narrow and cohesive — `ActivityEvent` and `HaltReason` in `state.rs`, and the activity types (`ActivityEvent`, `ActivityId`, `ActivityKind`, `ActivityOutcome`, `ActivityPhase`) in the other two files. All plain data types.
> That reframes option 2: rather than `cfg`-gating 21 sites, the coherent fix is to give those types a home `ui` can reach unconditionally (`trading_core`, which `ui` already depends on). **That is an architectural move and wants an ADR**, which is why it was not done unilaterally.

`crates/ui/Cargo.toml:230-232` documents the minimal build in its own words: *"Builds that explicitly want a minimal surface can opt out with `--no-default-features` and then re-enable individual features as needed (e.g. `--no-default-features --features fixtures` for the gallery-only bin)."*

`cargo check -p ui --no-default-features --features fixtures` fails with **three `E0432` unresolved-import errors**. `agent` is a dependency only under `live` (`live = ["dep:agent", …]`), but three modules import it unconditionally:

| file | `agent::` references |
|---|---|
| `crates/ui/src/state.rs:21` | **16** |
| `crates/ui/src/lab/activity.rs:18` | 3 |
| `crates/ui/src/widgets/activity_tape.rs:24` | 2 |

**Same family, one layer up.** This is the through-line applied to a *build configuration* rather than a function: a capability is **declared** (in the manifest, in prose, as a supported opt-out) and never **executed** (no CI leg, no script, no runbook builds it — `grep -rn -- "--no-default-features"` over `.github/`, `scripts/`, `.claude/skills/`, `README.md`, `docs/runbooks/` returns nothing outside archived prose). Nothing compares the two, so it rotted silently. The likely date of the rot is the same 2026-05-25 promotion that stranded #91's stub: once `live` became a default, every routine build carried `agent`, and the only configuration that would have caught this stopped being exercised.

**Why it blocks #91.** #91's fix — make `spawn_lab_run`'s `cfg(not(live))` arm return `Err` like both its siblings — lives in exactly this build configuration. It **cannot be compile-verified** until this builds. Applying it now would mean shipping an unverified edit to an arm no compiler has checked, which is the precise failure the `bar_span_hours` regression earlier in this session already cost two weeks. #91 is therefore **on hold**, not deferred by preference.

**Three honest resolutions, cheapest first:**
1. **Delete the claim.** If the gallery-only build is not actually wanted, stop documenting it in `Cargo.toml`. One comment edit; makes the manifest honest immediately.
2. **Fix it** — `cfg`-gate the three modules' `agent` usage under `live`. Real work: 21 references, 16 of them in core `state.rs`, so `cfg`-gated fields and message variants will ripple.
3. **Fix it, then add a CI leg that builds it**, so the configuration cannot rot again. Only option 3 actually closes the family — 1 and 2 both leave "declared vs executed" unchecked, they just change which side is true.

**Moral**: a feature-flag combination that nothing builds is untested code with a manifest comment promising otherwise. Feature *combinatorics* are a reachability surface in their own right — the audit that found #81 asked "is this feature on?" for single features; this is the question one level up, and `--no-default-features` is where it bites, because it is the one flag that turns things **off** that everything else assumes on.

### `#68-src` — the drift axis is INERT: `drift_rebalance_threshold` is parsed, range-validated, copied into a struct field, and never read
**Status**: source-confirmed 2026-08-17 (story 1-25 AC3 work, orchestrator). This is the source confirmation for the `#68` drift-axis item the 1-16 review routed into 1-25. Anchor-impacting: **the fix would be** — the axis currently is not.

**The chain, complete:**

| step | site |
|---|---|
| swept by the harness | `sweep_harness.rs:1721` — `cfg.drift_rebalance_threshold = cell.drift()` |
| **range-validated** | `config.rs:418` — errors unless in `(0, 1)` |
| copied to the strategy | `momentum.rs:194` — `drift_threshold: cfg.drift_rebalance_threshold` |
| declared | `momentum.rs:39` — `drift_threshold: Decimal` |
| **read** | **nowhere** — `grep drift_threshold momentum.rs` returns exactly those two lines |

So one of the three axes the Tier-1 grid advertises — *"lookback × k_long × drift_rebalance_threshold"* (`sweep_harness.rs:2684`) — has no consumer. The code takes care to **validate** a number it never uses, which is the tell: validation is the last place a value is touched before it disappears.

**Grid shape, measured:** 58 cells carry a drift value — **54 at `0.10`, one at `0.30`, three at `0.50`**. Since the value is never read, any two cells identical on the other axes are necessarily identical in output regardless of their drift labels.

**What I did NOT establish, stated so nobody inherits a false claim.** Whether the anchored grid actually *contains* such a pair needs a careful cell-by-cell read. A regex parse of the `ThetaCell` literals returned one candidate pair, but its own output was self-contradictory (both cells parsed as `k_long = 3` while their `role` strings read "top-1 only" versus "wide selection"), so the lookbehind was spanning neighbouring literals. **The parse is unreliable and its result is discarded.** The inertness above does not depend on it.

**Family.** Fourth declared-but-unread control in three days: **#85** (two account loss stops, zero read sites), **#69** (portfolio cap — enforcer exists, has passing tests, zero production callers), **#71** (per-symbol cap that capped the order rather than the position and could be walked past in increments), and now **#68**. In every case the value is *configured, documented, and often validated* — and never consulted. AC3's "enforce-or-delete + a BINDING test for every declared risk limit" is the correct response precisely because a fifth instance is more likely than not.

**Fix direction (operator's call, per the 1-16 review's ratify-or-fix framing):** implement the hold band so the axis does what the grid claims, **or** drop the axis and correct the narrative at regeneration. Do not leave it swept-but-inert: every θ-surface currently presents drift as an explored dimension. If it is implemented, a **drift-only cell pair becomes a mandatory per-axis divergence probe** — the same shape as AD-16's overlay gate, and the only thing that would have caught this.

### `#93` — `verify_anchors.sh` cannot see code-vs-evidence drift: it hashes committed bodies and never re-runs. The code stopped reproducing the frozen evidence, and 119/119 stayed green
**Status**: OPEN — known-red, routed to story **1-26** (found 2026-08-22 while fixing the CI test step). Anchor-impacting: **that is the finding**.

**What happened.** Four tests in `crates/backtest/tests/determinism.rs` (`t717_*`, `tt1_*`) RE-RUN a scenario from the pinned corpus and compare the resulting body-SHA to a literal. They are red:

```
top10-2023-1h-momentum
  expected 0f6f6eb8d943fefa866c4883be034f1beb3caff169fe76ec73bf3c29041a8ba3   (pinned; also in evidence/anchors.toml)
  got      b655e5e7f3edf1cec7c3e3c019876372b0d2840ee59028264e8ad891cabada15   (what the code produces today)
```

**And `bash scripts/verify_anchors.sh` reports `ANCHORS PASS (119 / 119)` at the same moment.** Both are correct, because they measure different things:

| gate | measures | can it see drift? |
|---|---|---|
| `verify_anchors.sh` | SHA-256 of the **committed report bodies** | **no** — the files are unchanged, so it passes forever |
| `determinism.rs` `*_anchor_hash_unchanged` | SHA of a **freshly re-run** scenario | **yes** — this is the only such gate in the repo |

So the corpus's own regression gate is structurally blind to the case where the *code* moves away from the *evidence*. The frozen bodies stay byte-identical; nothing re-derives them; the gate reports green. The four re-running tests are the closure for exactly that blind spot — and they are the ones that went red.

**They are telling the truth, and they predate this session.** Bisect against `83378c5` shows they failed **before** #67/#71/#75/#76 landed. Those fixes moved the numbers further; they did not cause the divergence. Something changed the harness's arithmetic earlier and the pins were never re-derived.

**Why they are now `#[ignore]`d rather than fixed or re-baselined.** Re-baselining a truthful regression gate to whatever the code currently emits is bug-log **#77**'s exact failure — it converts the gate into a rubber stamp and silently blesses whatever caused the drift. The pins can only be legitimately re-derived by the **1-26 re-lock**, which regenerates the affected surfaces under a new namespace and records per-scenario old-vs-new numbers in an errata. Until then they carry `#[ignore]` with the reason inline, CI verifies everything else, and they remain runnable:

```
cargo test -p backtest --test determinism -- --ignored
```

**⚠ THE CORRECTION BELOW WAS ITSELF WRONG — RE-CORRECTED 2026-08-23.** `git ls-files data/binance`
returns **exactly one path**: `REVISION.toml`. The *directory* is tracked; the *parquet corpus* is
not. So the original diagnosis was right and the correction was wrong — and removing the skip-guard
on the strength of it is what left `forecast::features::tests::windows_determinism_on_real_data`
panicking on every CI runner, red on all three legs, from 2026-08-22 until it was reproduced in a
clean clone and fixed 2026-08-23. The failure mode is worth naming: `root.exists()` was TRUE on the
runners *because* one metadata file is tracked, so the guard never fired while every byte it guarded
was absent. **A guard that checks a proxy for its condition is not a guard** — which is the sentence
this whole log keeps writing about other people's code.

**The superseded (wrong) text, kept because the mistake is the lesson:** The CI failure was first
diagnosed as "the machine-local corpus is absent on runners" and a skip-guard was written for it.
That premise is **false**: `data/binance` is **tracked**, so a runner has it. The gitignored — and therefore genuinely CI-absent — data dirs are `audit`, `audit.db`, `binance-dynamic`, `reflection`. The guard was removed rather than left in place, because a guard documenting a condition that never occurs is itself the declared-vs-actual defect this log exists to catch.

**Moral**: an immutability gate is not a reproducibility gate. Hashing what you stored proves only that storage is intact; it says nothing about whether the producer still produces it. If the evidence is meant to be *reproducible*, something must actually re-run — and that something must be allowed to fail.

**Characterised in plain units 2026-09-24** (while scoping `#107`), which is more legible than
the hash comparison above and worth having in the entry. At clean `HEAD`, no local
modification, the four synthetic `btc-2023-1m-*` scenarios re-emit as:

| `btc-2023-1m-sma-cross` | committed body | fresh run |
|---|---|---|
| Bars replayed | 525601 | 17544 |
| Final equity | $47290.03 | $107381.95 |
| Total fees | $33435.478506 | $1849.150109 |

**525600 is the minutes in a 365-day year. 17544 is 731 × 24 — the hours in two years.** So a
scenario named **`1m`** now produces **hourly** bars over a **two-year** span. Not a rounding
drift: a different cadence over a different window, and the equity is 2.3× the recorded value
with fees 18× smaller. The other three scenarios diverge identically.

Consequence beyond this entry: **no anchor in this family can be re-locked until this is
resolved**, because a re-emission would freeze today's computation and silently retire the
recorded one. That blocks `#107`'s fix, and it is why story 1-26 owns both.

### `#94` — `size_portfolio_target` sized a resize order to the whole TARGET, not the delta. Wiring it lost 74 % of equity on the first fixture through the resize path
**Status**: FIXED 2026-08-23 (`crates/risk/src/portfolio.rs`), binding test
`crates/backtest/tests/portfolio_controls_bind.rs::a_resize_converges_instead_of_overshooting_every_bar`.
Found while wiring ADR-0089 D1.

**What happened.** The open/resize branch emitted an order for
`|target_notional / mark|` — the full target quantity — regardless of what the
leg already held. That is correct only against a *"set position to X"* venue
API. Every execution path in this repository fills INCREMENTALLY:
`PaperEngine::step` ADDS the fill to the book. So a resize of a leg already
holding 10 % of equity ordered another 10 %, then another, until cash was gone.

**How it surfaced.** `horizon_divergence_e2e::f_hr_4_signal_non_no_op_daily`
went red the moment the sizer was actually called, with TS and always-long
returning *byte-identical* results — the tell that neither strategy's decisions
were reaching the book any more. Instrumented: final equity **25 722** from
100 000 (−74 %) with `min_cash_seen` at **43.8**. The book had gone fully
invested on a fixture that targets 10 %.

**Why it survived.** Two independent reasons, and they compound:

1. **No caller.** This is #69 — the function had no production call site, so the
   resize path had never executed anywhere.
2. **Its own tests could not see it.** All 13 pre-existing unit tests exercise
   flat → open or → close. In BOTH, delta and target coincide exactly. The one
   case where they differ — a same-side resize of a held leg — had no test.

**The sharper consequence: #94 disabled #68.** A delta-sized order lands the leg
ON its target, so the drift band then holds it for several bars. An
absolute-sized order OVERSHOOTS, so the leg is outside the band again on the very
next bar and re-trades forever. Measured on the binding fixture (60 bars, 0.10
band, +2 %/bar): **10 fills delta-sized vs 50 absolute-sized**. Had the sizer
been wired without fixing this, the hold band would have been switched on and
observably suppressed nothing — and the natural conclusion would have been that
the band does not work, rather than that the orders never converged.

**Also corrected here:** `total_gross_notional` accumulates `target_notional`
(the RESULTING leg), never the delta — the cap is a limit on the book after the
rebalance, not on its turnover.

**Moral**: "returns a `Vec<Order>`" does not say whether an order is an absolute
position or a delta, and the two are indistinguishable in every test where the
prior position is zero. An order-producing function needs at least one test
whose starting position is non-zero and whose target is non-zero — otherwise the
entire semantic is unpinned.

### `#95` — `portfolio_exposure_cap` is declared at NINE sites and read at ONE. Wiring `run_path` fixes one lane; eight others still declare a cap they cannot enforce
**Status**: OPEN — scope finding, needs an operator ruling. Found 2026-08-23 while
closing #69.

**The census, whole-workspace and reproducible:**

```bash
# declarations
grep -rn "portfolio_exposure_cap: Some" crates/*/src/     # 9 sites
# reads
grep -rn "portfolio_exposure_cap" crates/*/src/ | grep -v "portfolio_exposure_cap:"   # 1 site
```

The single read is `crates/risk/src/portfolio.rs:243`, inside
`size_portfolio_target`. `Order::new` does NOT consult it — its cap is the
per-symbol one. So the portfolio cap is enforceable through exactly one function,
and until 2026-08-23 that function had no production caller (#69).

**What the wiring did and did not fix.** `run_path` now calls the sizer, so the
cap binds there — and `run_path` is the lane behind **every one of the 34 anchors
in the 1-25 inventory** (#86-#119 are all θ-surfaces from
`param_robustness_sweep`, which routes through it). The other eight declaring
lanes still construct orders per signal and therefore still cannot enforce it:

`patchtst_overlay_weights` · `threshold_sweep` (the TCN τ/ε `run_cell`, distinct
from `run_path`) · `tcn_overlay` · `garch_vol_target_overlay` ·
`tcn_overlay_weights` · `pairs` (declares **0.75**) · `momentum` ·
`regime_dispatcher`

**A correction to the 1-25 architect note.** It records "two production lanes are
implicated (`run_path`, `run_cell`)". For the 34-anchor inventory that is one
lane too many: `scenarios/threshold_sweep::run_cell` is the candle-gated TCN
threshold sweep and produces none of #86-#119. The inventory is entirely
`run_path`, which is why D1 could be discharged without touching `run_cell`.

**Why this is not just "more of #69".** #69 was one uncalled function. This is the
declaration side: eight call sites that state a limit in a struct the code path
they use never inspects. Fixing them means either routing those lanes through the
sizer too (a repeat of the D1 restructure, eight times, on lanes with their own
anchors) or deleting the declarations and saying so in the reports that print
them. **Neither should be chosen silently**, which is why this is logged rather
than fixed.

### `#96` — the P0-1 "show your work" credibility block renders ENTIRELY below the fold, and its pixel gate has been measuring the scrollbar
**Status**: FIXED 2026-09-16 — ADR-0092 (placement) + a rewritten gate. Found 2026-08-23
while piloting #88; bisected. Anchor-impacting: **no** (cockpit surface).

**The gate.** `leaderboard_scorecard_render` renders the leaderboard with
`scorecard = Some(..)` and against a control with `scorecard = None`, and asserts the first paints
STRICTLY MORE foreground. It fails:

```
with=39794 vs without=41774, delta=-1980
```

**Bisected to `67f2a9d`** (`feat(advisor-data-quality-surface): P1-7 — DATA-stage trust/quality
panel`), which added a large Data quality panel to the same screen. The test passed at `00240ed` and
fails at every commit after `67f2a9d`.

**Diagnosed at the pixels, not inferred.** The two 1920x1080 renders were diffed:

```
differing region bbox (l,t,r,b): (1894, 820, 1904, 1020)   -> 10 px wide, at x=1894 of 1920
differing pixels: 1986
```

A 10-pixel-wide strip at the right edge of the viewport is the **SCROLLBAR**. Everything else is
byte-identical. So:

1. **The credibility block contributes ZERO visible pixels.** It is entirely below the fold.
2. **The delta the gate measures is the scrollbar thumb.** Adding content below the fold lengthens
   the scroll extent, which SHORTENS the thumb, which REMOVES foreground pixels — hence a
   *negative* delta. The gate is not weakly measuring the feature; it is measuring the scrollbar,
   with the opposite sign.

**Why the fixture did not save it.** The test's own doc records that it deliberately uses the 2-row
`benchmark_wins` fixture "so the scorecard block fits". That short-table trick bought a fixed
vertical budget — and `67f2a9d` spent it. The safeguard was real, and a later feature silently
consumed it.

**The product consequence is the serious half.** This is the P0-1 overfitting scorecard — the block
whose entire purpose is telling the operator how much to trust the ranking. On a 1920x1080 viewport
it is now off-screen; a user sees the recommendation and the table without ever meeting the
"how much to trust this" panel unless they scroll. The honesty surface is the thing that got pushed
below the fold.

**Why nobody saw the red.** Two independent maskings, both now fixed: the macOS canonical UI step was
being CANCELLED by an earlier failing step (so it reported `skipped`, never `failure`), and a red run
was undiagnosable without repo admin. Both are addressed — but note the ordering: this regression has
been live since `67f2a9d` and no CI run has ever reported it.

**Fix direction — a REAL decision, deliberately not taken here.**
- **(a) Move the credibility block above Data quality.** Changes what users see, which is the actual
  complaint. Restores the gate's meaning for free.
- **(b) Make the harness scroll to the block, or render a taller viewport.** Restores the gate only.
  If chosen, say so explicitly: it accepts that the block lives below the fold.

Do NOT "fix" this by loosening the assertion or re-baselining the delta — the number it compares is
the scrollbar, so any threshold fitted to it is fitted to noise. That is bug-log #77's failure.

**Fixed (2026-09-16, ADR-0092)** — option (a), with one measured correction. The scorecard
block renders FIRST in the ready pane, and the pick follows it: leading with BOTH honesty
panels (scorecard then Data quality, the literal ruling) was implemented, rendered, and
measured to push the recommendation and the entire ranked table below the fold — the same
bug pointed at the pick. Shipped order: scorecard -> recommendation -> table -> Data quality
-> Risk story, so the trust readout, the pick and the first ranked rows all clear 1080. The gate was
rewritten rather than re-tuned: presence is measured in a 1920x2400 frame where nothing scrolls
or clips (asserted, not assumed), so the with/without delta is the block itself; visibility is a
separate assertion that locates the block's extent and requires it to end above y=1080; and no
measurement reads the 32-px scrollbar gutter, so no gate on this screen can be satisfied by
chrome again.

**Moral**: a pixel gate that compares two renders can be satisfied by ANY difference between them,
including chrome the feature never touches. When a harness reserves a resource by construction — a
short fixture, a fixed viewport — that reservation is an invariant with no enforcement, and the next
feature to want the same space will take it silently.

### `#97` — AD-2's byte-immutability does not survive a Windows checkout: nothing constrained line endings, so git rewrote the anchored corpus
**Status**: **FIXED 2026-08-29** (`.gitattributes`), proven in both directions. Found while
diagnosing the windows-latest leg. Anchor-impacting: **no** — the anchored BLOBS are unchanged; what
changed is a guarantee that they arrive intact.

**The claim.** AD-2 pins 119 report bodies by SHA-256 and requires them byte-identical before and
after any change. The SHA is taken over the file's BYTES — `read_to_string` -> strip front matter ->
`hash(body.as_bytes())`, with no `\r` handling anywhere (`strategy_anchors_unchanged.rs:402`).

**The gap.** `.gitattributes` carried exactly two lines (an LFS filter and one `binary` rule) and no
text/eol rules at all, so `evidence/**` resolved to `text: unspecified`. Git then falls back to
`core.autocrlf` — which is `true` on Windows and on GitHub's `windows-latest` runner.

**Measured, not argued.** Cloning this repo with `-c core.autocrlf=true` and reading one anchored
report:

| | CRLF | bare LF | bytes |
|---|---|---|---|
| Windows-style checkout | **81** | 0 | **3103** |
| LF checkout (canonical box) | 0 | 81 | **3022** |

Different bytes, therefore a different SHA — confirmed directly:
`552d7df2…` (LF) vs `0f4c4bad…` (CRLF) on the same report. **All 119 anchors fail on such a
checkout**, and `t1937_nine_strategy_anchors_unchanged` /
`t1937b_canonical_strategy_anchors_unchanged` on the windows leg are exactly that.

**A second consequence, found by running the gate in that clone.** The gate SCRIPTS get CRLF too:

```
scripts/verify_anchors.sh: line 58: set: pipefail: invalid option name
```

`pipefail\r` is not an option name. So on a Windows checkout the anchor gate does not merely fail —
**it cannot execute**.

**Why it looked safe.** `verify_anchors.sh` runs on the ubuntu leg only. A byte-immutability
invariant whose truth depends on which OS performed the checkout is not an invariant; it is a
property of one machine.

**The fix**, verified end-to-end in throwaway clones rather than reasoned about:

```
* text=auto eol=lf          # normalize everywhere, repo and working tree
evidence/** -text           # byte-immutable: never translate, either direction
.claude/skills/bmad-brainstorming/**/*.csv -text   # the only 2 tracked files with CRLF blobs
```

- Windows-style checkout WITH the rules: **0 CRLF, 3022 bytes** — identical to the canonical box.
- `verify_anchors.sh` in that same clone: **ANCHORS PASS (119 / 119)**.
- The two vendored CSVs keep their CRLF (109 each) — `text=auto` would otherwise rewrite them.
- On this machine the change produces **no re-normalization churn**: `.gitattributes` is the only
  modified path.

**Moral**: this repository hashes its own artefacts and calls the result an invariant, while leaving
the delivery of those bytes to a client-side config it never set. The compiler had the same hole
(ADR-0091) and was closed the same week. When you pin an output, pin every input that can change it —
and note that both holes were found only because a CI leg finally ran.

### `#98` — R13.3's 256 MiB RSS budget is enforced with a ruler that reads 5x differently per platform
**Status**: OPEN — needs a ruling on R13.3, not a patch. Found 2026-08-29 once CI failures became
readable. Anchor-impacting: **no**.

**Measured, same test, same workload, same commit:**

| platform | peak `ru_maxrss` | verdict vs `< 256 MiB` |
|---|---|---|
| macOS (canonical box) | **54.3 MiB** | PASS, with 4.7x headroom |
| ubuntu-latest (CI) | **269.6 MiB** | FAIL by 5 % |

`t815_perf_smoke_90d_under_10s_and_under_256mib` (`crates/reports/tests/perf_smoke.rs:155`). The
wall-clock half of the same test passes comfortably on both (1.06 s against a 10 s budget here).

**The unit handling is CORRECT — that was checked first.** `peak_rss_bytes()` branches on
`cfg(target_os)`: bytes on macOS, kilobytes x 1024 on Linux (`perf_smoke.rs:66-76`), exactly as the
platforms define `ru_maxrss`. Both numbers above are genuine bytes. And the measurement is isolated:
the binary contains a single test, so `RUSAGE_SELF` peak is not polluted by siblings.

**So the gap is real, and 5x is far too large to read as "the code needs 270 MiB".** The same code
doing the same work reports 54 MiB under one kernel's accounting and 270 MiB under another's.
`ru_maxrss` counts resident pages, and what lands resident differs by allocator (glibc arenas vs
macOS libmalloc), by whether parquet reads are mapped or copied, and by kernel reclaim policy. It is
not a portable measure of what the program *needs*.

**The defect is therefore not the number — it is that a declared requirement is enforced by an
instrument that is not comparable across the platforms it now runs on.** R13.3 says "RSS < 256 MiB".
That budget was calibrated on the box where the metric reads ~5x lower, and the 3-OS matrix has been
silently applying it to a ruler nobody re-calibrated. Same family as the rest of this log: the
measurement is a proxy, and the proxy changed underneath the claim.

**Why it surfaced only now.** It has presumably failed on every ubuntu run since the matrix was
activated, but the leg died earlier (build, then `compile_fail_tests`, then corpus guards), and run
logs need repo admin. It became visible in one line the moment the annotation tooling landed.

**Options, none of them "raise the number and move on":**
- **(a) Platform-aware budget** with the calibration recorded per platform, and a note that the two
  numbers are not comparable. Honest, cheap, and keeps the gate live on all three legs.
- **(b) Measure something comparable** — e.g. peak allocated bytes via an instrumented allocator —
  so one budget means the same thing everywhere. Correct, more work.
- **(c) Scope R13.3 to the canonical box** and say so in the requirement, leaving the other legs to
  assert the wall-clock half only. Weakest, but honest if the budget is really a macOS claim.

**Do NOT simply raise 256 to 300.** That fits the number to the noisiest platform, silently
re-baselines a declared requirement to whatever CI happens to emit, and leaves the next platform to
break it again — bug-log #77's failure applied to a performance budget.

### `#99` — the audit aggregator silently DISCARDS the final window's count when the tick bus closes
**Status**: OPEN — product-side, found 2026-08-29 while fixing the storm test's accounting.
Anchor-impacting: **no**.

`run_aggregator_loop` accumulates events into `agg.counter` and emits that count at each 100 ms
`interval.tick()` boundary via `swap(0)`. On `RecvError::Closed` it `break`s and then:

```rust
// Explicitly drop handle so End{Success} is emitted before the task exits.
drop(agg.handle);
```

Whatever sits in `agg.counter` at that moment — every event received since the last boundary — is
never emitted. Not as a `Tick`, not in the `End` event. It is dropped.

**Measured.** With the storm test's accounting made exact (Start-label count + Tick counts), a clean
run reconciles perfectly:

```
start_label=62 + ticks=4938 = 5000/5000 (100.0%)
```

Under load, where the close lands mid-window:

```
start_label=3 + ticks=3650 = 3653 / 5000  (73.1%)   <- 1347 events unaccounted
```

The gap is exactly the un-flushed final window.

**Why it matters beyond a flaky test.** The activity tape is an operator-facing honesty surface: it
reports how much the audit ledger is writing. Under-reporting the tail of every burst means the
number the operator reads is systematically low whenever a run ends mid-window — which is the normal
way runs end. The undercount is invisible because nothing reconciles the tape against the ledger.

**The fix is small and local** — before `drop(agg.handle)`, flush a non-zero counter through the
existing handle:

```rust
let n = agg.counter.swap(0, Ordering::Relaxed);
if n > 0 && let Some(h) = agg.handle.as_ref() { h.tick(n as u64); }
drop(agg.handle);
```

**NOT applied here.** It changes PRODUCT behaviour on the activity tape (one extra `Tick` at
shutdown), and the operator's 2026-08-29 ruling on the sibling question deliberately chose the
test-side options over the product-side one. This is a different product change from the one that
was declined, so it wants its own answer rather than being folded in silently.

**Until then** the storm test is materially better but not perfect: 9 of 10 passes under 42 CPU
burners, against 2 of 5 FAILURES at unmodified HEAD unloaded. The residual failure is this defect,
and the assertion message now names the split (`start_label=… + ticks=…`) so the next occurrence is
self-diagnosing.

## Changelog

- 2026-08-29 (orchestrator): **#99 added (OPEN, product-side)** — the audit aggregator DISCARDS the final window's count when the tick bus closes: `run_aggregator_loop` breaks on `Closed` and drops the handle without flushing `agg.counter`, so every event since the last 100 ms boundary vanishes from the activity tape. Found by making the storm test's accounting exact: a clean run now reconciles at **5000/5000 (100 %)**, while a run whose close lands mid-window reads **3653/5000 (73 %)** — the gap is precisely the un-flushed window. It matters beyond the test: the tape is an operator-facing honesty surface, and runs normally end mid-window, so the reported write count is systematically low at the tail with nothing reconciling it against the ledger. The fix is ~3 lines before `drop(agg.handle)` but changes product behaviour (an extra shutdown `Tick`), and the same-day ruling deliberately chose test-side over product-side on the sibling question — so it is recorded for its own decision rather than folded in.
- 2026-08-29 (orchestrator): **storm test accounting FIXED per the operator ruling.** `TOTAL_EVENTS` 10 000 -> 5 000 (below the aggregator's `K2_THRESHOLD` of 9 999, so the Start label stays parseable), and coverage now sums the Start-label count with the Tick counts — the first non-empty window emits NO Tick by design, so its events were previously invisible to the 90 % bar. Also removed the last scheduling assumption: `tick_count >= 1` required the burst to span two windows by luck (it does not, under load, 4 of 8 runs) and is replaced by `start_count >= 1` plus the accounting bar. Result: 9/10 under 42 CPU burners vs 2/5 FAILING unloaded at HEAD. Chunking the sends across windows was tried and REVERTED — it measured worse (65 %), because more windows means more chances to lose a partial one to #99.
- 2026-08-29 (orchestrator): **#98 added (OPEN — needs an R13.3 ruling).** `t815_perf_smoke_90d_under_10s_and_under_256mib` measures **54.3 MiB on macOS and 269.6 MiB on ubuntu** for the same workload — a 5x gap, so the 256 MiB budget passes with 4.7x headroom on the calibration box and fails by 5 % in CI. The unit handling was checked FIRST and is correct (`cfg(target_os)`: bytes on macOS, KB x 1024 on Linux), and the binary holds a single test so `RUSAGE_SELF` is not polluted. The defect is not the number: a declared requirement is being enforced by an instrument that is not comparable across the platforms it now runs on, calibrated on the one that reads lowest. Options are a platform-aware budget, a comparable metric (instrumented allocator), or scoping R13.3 to the canonical box — explicitly NOT raising 256 to 300, which would fit a declared requirement to the noisiest platform (#77's failure in performance clothing). Visible only now: the ubuntu leg previously died earlier, and logs need repo admin.
- 2026-08-29 (orchestrator): **#97 added and FIXED — AD-2's byte-immutability did not survive a Windows checkout.** `.gitattributes` had no text/eol rules, so `evidence/**` fell back to `core.autocrlf`, which is `true` on windows-latest. Measured: the same anchored report checks out as **3103 bytes / 81 CRLF** on Windows vs **3022 / 0** on the canonical box — `552d7df2…` vs `0f4c4bad…`. All 119 anchors fail on such a checkout; `t1937*` on the windows leg is exactly that. Worse, the gate SCRIPTS get CRLF too, so `verify_anchors.sh` dies with `set: pipefail: invalid option name` — the gate cannot even run there. It looked safe only because the gate runs on ubuntu alone. Fixed with `* text=auto eol=lf` + `evidence/** -text` + `-text` on the two vendored CRLF CSVs; verified in throwaway clones BOTH ways (Windows-style checkout now 0 CRLF / 3022 bytes, and `ANCHORS PASS 119/119` inside that clone), with zero re-normalization churn locally. Same shape as ADR-0091's unpinned compiler, found the same week, and both surfaced only because a CI leg finally executed.
- 2026-08-23 (orchestrator): **the same proxy-probe defect found in THREE more files, and it was red on two CI legs.** `crates/ui/tests/lab_binance_{divergence,persist_compare,render}.rs` all guarded on `data/binance/REVISION.toml.is_file()` — the ONE tracked path under `data/binance` — as a stand-in for the gitignored parquet corpus. On every runner the probe is TRUE, the skip never fires, `preload` errors, and the tests panic through an arm that literally reads *"corpus PRESENT ... hard FAIL, not a skip"*. Four sites now share one root cause (the fourth was `forecast::features`), and in each the guard checked a proxy rather than the condition. The probes now name the months the loader actually reads (`data/binance/BTCUSDT/2023/01..06.parquet`). **Verified in BOTH directions, which is the part that matters for a skip-guard:** corpus present -> the tests RUN for real (3/1/3 passed); corpus absent (clean clone = the CI condition) -> visible `[skip]` lines, no panic. A guard that only ever skips is the same defect in a new coat. These failures were invisible until the UI steps stopped being cancelled by an earlier failing step (6753ff1) and CI failures became readable without repo admin (7c126db).
- 2026-08-23 (orchestrator): **#96 added (OPEN — needs a product decision).** The P0-1 "show your work" credibility block renders ENTIRELY below the fold, and `leaderboard_scorecard_render` has been measuring the SCROLLBAR. Bisected to `67f2a9d` (the DATA-stage quality panel), which spent the vertical budget the test had bought with a deliberate 2-row fixture. Diagnosed at the pixels: the two 1920x1080 renders differ in exactly one 10px-wide strip at `x=1894..1904` — the scrollbar — for 1986 pixels, matching the reported `delta=-1980`. Adding content below the fold lengthens the scroll extent, shortens the thumb, and removes foreground, so the gate reads NEGATIVE. The block contributes zero visible pixels. Two maskings kept it invisible (the macOS UI step was cancelled by an earlier failure; logs need repo admin), both now fixed — so no CI run has ever reported a regression that has been live since `67f2a9d`. Fix is (a) move the block above Data quality or (b) scroll/enlarge the harness and state that the block lives below the fold; NOT re-baselining a threshold fitted to scrollbar noise (#77).
- 2026-08-23 (orchestrator): **CI was RED on all three legs since 2026-08-22, and it was my own #93 correction that did it.** The failing step is `cargo test --workspace --exclude ui`; it PASSES locally and fails on any fresh checkout, so it was reproduced in a clean `git clone` — one test, `forecast::features::tests::windows_determinism_on_real_data`. Root cause: `git ls-files data/binance` returns **exactly one path** (`REVISION.toml`). The DIRECTORY is tracked; the PARQUET CORPUS is not. So `root.exists()` is TRUE on every runner while every byte the test reads is absent — the skip never fired, `windows_for_symbol` returned `Err`, and the test panicked. **#93's note claiming "the CI failure is NOT missing corpus — `data/binance` is TRACKED and present on runners" is now marked wrong in place**, along with the skip-guard removal it justified. Fixed by guarding on the two parquet files the test actually reads and emitting a VISIBLE `[skip]` line (bug-log #66). Verified both ways: skips in the clean clone, runs for real locally.
- 2026-08-23 (orchestrator): **#82 RE-MEASURED — mechanism 1 FIXED by #71, gate added and RED-proven; point 3 RETRACTED.** `v0.sma_cross_ls` now takes **128 / 262 short legs** (was 0 / 0) with `max_pos` down from **28.2 / 39.2 units to 1.83 / 0.98** and terminal equity positive on both windows (was −9 235 / −14 146). The leverage ratchet is gone: the arm returns to flat, so the short entry fires. `assert_no_ratchet_and_shorts_taken` is the assertion #82's own moral asked for; restoring the side-blind cap fails BOTH windows with `took 0 short legs`, reproducing the original measurement exactly. **Mechanism 2 unchanged and still open** — `macd_ls`/`rsi_ls`/`bbands_ls` still alternate perfectly and take zero shorts, exactly as #82 predicted, and the gate deliberately does not assert on them. **Point 3 was wrong**: `v0.always_short` dispatches to `run_alwaysshort_path`, a closed-form equity path that emits no `Fill` by construction — `v0.buyhold` reports `fills=0` in the same table, and the sanity gate shows the arm returns 100 000 → 156 210 on the bear window. Reasoning from a symptom without tracing the implementation, in the one entry that already self-corrected for it once.
- 2026-08-23 (orchestrator): **#95 added (OPEN — needs a ruling)** — `portfolio_exposure_cap` is declared at **9** sites and read at **1** across the whole workspace; the single read is inside `size_portfolio_target`, and `Order::new` never consults it. Wiring `run_path` (#69) binds the cap on the lane behind **all 34** inventory anchors — #86-#119 are θ-surfaces from `param_robustness_sweep`, which routes through it. Eight other lanes still declare a cap they cannot enforce (`pairs` declares 0.75). Also corrects the 1-25 architect note: `scenarios::threshold_sweep::run_cell` is the candle-gated TCN τ/ε sweep and produces NONE of the inventory anchors, so D1 was dischargeable on `run_path` alone.
- 2026-08-23 (orchestrator): **#94 added and FIXED — the sizer's resize order was the TARGET, not the delta.** Found by wiring ADR-0089 D1: `size_portfolio_target` emitted `|target_notional / mark|` on every action, correct only against a "set position to X" API, while every path here fills incrementally. A leg targeted at 10 % of equity accumulated another 10 % per rebalance — measured **−74 % equity, `min_cash_seen` 43.8 / 100 000**, with TS and always-long returning byte-identical results. Survived because #69 meant no caller AND because all 13 of its unit tests start from a flat position, where delta and target coincide. **It also disabled #68**: an overshooting order leaves the leg outside the band every bar, so the hold band could never have held anything (10 fills delta-sized vs 50 absolute-sized on the binding fixture). Fixing it first is what let #68's gate be RED-proven at all.
- 2026-08-23 (orchestrator): **#68 + #69 CLOSED — the sizer is wired and both controls now BIND.** `run_path` builds a signed target vector at each rebalance boundary and calls `size_portfolio_target` (ADR-0089 D1); breaches skip the whole rebalance, increment `PathRunResult.portfolio_breaches`, and are logged (D2). Three RED-proven gates in `portfolio_controls_bind.rs` — neutering the cap, the band, or #94's delta sizing each turns exactly one of them red. **Two corrections to ADR-0089 recorded with the fix:** (1) the gate is the REBALANCE BOUNDARY, not `!signals.is_empty()` — signals are a delta, so a full book emits none and a signal-gated rebalance would have left the band nearly as inert as #68 found it; (2) the ADR's "turnover falls" is **wrong** — the old code could never resize a held leg at all (`Buy if current_qty <= 0`), so the band does not suppress old behaviour, it bounds NEW behaviour, and the net direction is an empirical question for 1-26.
- 2026-08-22 (orchestrator): **#93 added (OPEN, known-red → 1-26)** — `verify_anchors.sh` hashes COMMITTED report bodies and never re-runs, so it cannot see the code drifting away from the evidence: it printed `ANCHORS PASS (119/119)` while four re-running determinism tests showed the same scenario now hashes `b655e5e7…` against a pinned `0f6f6eb8…`. Those four are the ONLY gate in the repo that can observe code-vs-evidence drift, and they are correctly red; bisect puts the divergence BEFORE #67/#71/#75/#76. `#[ignore]`d with the reason inline (still runnable via `-- --ignored`) rather than re-baselined, because re-pinning a truthful gate to current output is #77's failure. Also corrects a wrong diagnosis: the CI failure is NOT missing corpus — `data/binance` is TRACKED and present on runners; the gitignored dirs are `audit`, `audit.db`, `binance-dynamic`, `reflection`. The skip-guard written on that false premise was removed.
- 2026-08-22 (orchestrator): **#69 UNITS RULED — `exposure_cap` MEANS GROSS (Σ |notional|)**, ADR-0089 D7. This settles a question the corpus never answered and it lands against the surfaces: at 6 legs × fraction 0.10 the MN book runs **0.60 gross vs its hashed `exposure_cap = 0.50`**, so **the anchored MN surfaces DID breach their own declared limit** — #69's reading is now the official one, not a candidate. Net was rejected as near-vacuous (≈0 by construction on a market-neutral arm, so the cap could never bind on the very lanes it was written for); long-only was rejected because it ignores half the book. **Consequence for the fix: `size_portfolio_target` cannot implement the ruling as written** — it caps `total_long_notional`, the long-only measure that was just rejected — so it must be extended to signed weights with a gross cap, or replaced. The second-order consequence is the sharper one: surfaces that previously *reported compliance* were non-compliant under the ruled measure, so 1-26's errata owes a per-scenario record, not just new numbers.
- 2026-08-22 (orchestrator): **#92 RULED — fix the cfg gating AND add a CI leg.** Relocate the shared types (`ActivityEvent`, `HaltReason`, the activity enums) into `trading_core`, which `ui` already depends on unconditionally, then add a CI job that builds the minimal config so it cannot rot again. Chosen over deleting the claim or fixing without coverage: it is the only option that closes the declared-vs-executed family rather than changing which side happens to be true. Unblocks **#91**, whose fix lives in exactly that build. Needs an ADR for the type relocation.
- 2026-08-17 (orchestrator): **#68 SOURCE-CONFIRMED** — the drift axis is inert. `drift_rebalance_threshold` is swept (`sweep_harness.rs:1721`), **range-validated** (`config.rs:418`), copied to `momentum.rs:194`'s `drift_threshold` field — and read NOWHERE (grep returns only the declaration and the assignment). One of the three advertised Tier-1 grid axes has no consumer; the code validates a number it never uses. Grid measured: 58 cells, 54 at 0.10 / 1 at 0.30 / 3 at 0.50. NOT established: whether a drift-only cell pair exists in the anchored grid — a regex parse returned a self-contradictory candidate and was discarded. Fourth declared-but-unread control in three days (#85, #69, #71, #68).
- 2026-08-15 (orchestrator): **#92 interim applied** — `crates/ui/Cargo.toml` no longer advertises `--no-default-features` as supported; the comment now states it does not build, names the three files, and records that no shipping target needs it. Scope re-measured and it is SMALLER than first reported: most of `state.rs`'s 16 `agent::` mentions are doc comments, so the real surface is `ActivityEvent` + `HaltReason` + the activity types — one cohesive group of plain data types. That makes the coherent fix a relocation into `trading_core` (which `ui` already depends on unconditionally) rather than cfg-gating 21 sites — an architectural move that wants an ADR, hence not done unilaterally.
- 2026-08-15 (orchestrator): **#90 OPTION 1 APPLIED** — the carve-out is now documented at the definition (no `Fill`, both parity gates blind by construction, why it is symmetric, why engine-routing was deferred) and **gated** by a new caller census, `liquidation_carve_out_census.rs`. A fill-tape gate can never see this path, so the caller set is the only thing that can be guarded — the test locks it in both directions (no new callers; no stale allow-list entries) and carries a >100-files sanity assert so a broken walker cannot pass vacuously. RED-proven by planting a third caller; the failure names the file and explicitly forbids just adding it to the allow-list. Options 2/3 still the operator's.
- 2026-08-15 (orchestrator): **#89 PARTLY FIXED — the predicted tautology existed and is gone.** `t24_deterministic_across_runs` passed the SAME seed to both engines and asserted equal fills; RED-proven vacuous by mutating one seed to 999_999 (still passed), then replaced with deliberately DIFFERENT seeds asserting fills are seed-INDEPENDENT — true today, falsifiable the moment anyone wires `self.rng`, and annotated to say the anchor story must move with it. Field now carries a ⚠️ INERT doc comment. 225 backtest lib tests pass, clippy/fmt clean, anchors 119/119 (checked — paper.rs is on the fill path). Wire-or-delete still open.
- 2026-08-15 (orchestrator): **#85 interim FIX applied; one evidence leg CORRECTED.** Correction: the runbook documenting both stops as kill-switch triggers is in `docs/archive/` — FROZEN history, not a live doc — so that leg is weaker than the entry implied (the live config files and defaults are unaffected; the archived file was deliberately not edited). Applied the entry's own recommended interim: ⚠️ NOT ENFORCED doc comments on both `RiskConfig` fields plus header comments in `config/agent.toml` and `.soak-fast`, with `per_symbol_exposure_cap` documented as ENFORCED for contrast — clippy `-D warnings` exit 0, fmt clean, 29 config tests pass, zero behavioural change. Sizing for the real fix: `KillSwitch::trip` already exists and the forward loop already computes `cur_equity`; what is missing is two `HaltReason` variants — the enum has six and neither stop is among them, which is the defect in one line. Wiring NOT applied: it makes runs start halting, which is the operator's call.
- 2026-08-15 (orchestrator): **#83 SHARPENED + staged for ruling** — decision memo at [`decision-83-frozen-gate-fails-open-2026-08-15.md`](decision-83-frozen-gate-fails-open-2026-08-15.md). Three things changed the framing. (1) **The correct treatment already exists in-tree**: `sweep.rs:1159-1174` maps the same `None` from the same function to **Fragile** — *"a curve too short to score is untrustworthy"* — so this is two call sites disagreeing, not a new policy. (2) **That comment's claim is FALSE**: it says this is *"consistent with Skipped→Fragile in the leaderboard context"*, but in the leaderboard `Skipped` is crown-ELIGIBLE, the opposite. A documented belief nothing checked — the session's through-line, inside the frozen gate's own neighbourhood. (3) **The fix is one file, one function**: `Skipped` is NOT overloaded as feared — the intentional skip is `c.robustness == None` (mod.rs:1332/1419) while attempted-but-failed is `Some(Skipped)`, already distinct, so `bootstrap.rs` needs no edit and only `rank.rs::is_eligible` changes. Recommended patch is the **allow-list** (a future flag variant then fails CLOSED; the deny-list would silently re-open this). **Severity honestly undetermined**: none of the 62 anchored reports render the robustness flag at all (`grep -i robust|fragile|marginal` → 0 hits), so the corpus cannot witness whether it has fired; structurally it needs a <2-point equity curve, so probably latent. NOT applied — `rank.rs` is AD-1 byte-frozen and this changes crown eligibility.
- 2026-08-15 (orchestrator): **#86 FIXED** — resolved in the order that keeps the gate green: (1) wrote `0079-shared-vol-estimator.md` as a reconstruction from primary sources only (registry row + the module doc-comment that carries the design decisions verbatim + ADR-0078 + architecture §6.0 D5), labelled as such, corpus now 87 files = 87 rows; (2) added bidirectional invariant **(d) decision-file-missing**, RED-proven by hiding the file and restoring it; (3) fixed a second defect found inside the gate — `--self-test` exercised `_check_invariants_raw`, a near-identical COPY of the `_check_invariants` that production calls, so the lint's own tests did not guard the lint's own code (#77's shape, one level in); the two are now one implementation. Self-test 5 → 7 cases. The new check promptly flagged a latent inconsistency in case 1's own fixture. The written ADR additionally records that 3 of its 4 functions and 1 of its 2 declared consumers have no production caller.
- 2026-08-15 (orchestrator): **#87 HALF-FIXED** — the silent off-path is gone. The `cfg(not(forecast-audit-tick))` arm in `agent/runtime.rs` now warns loudly, with the rebuild command, when the documented `[strategies.tcn_overlay_momentum] enabled = true` is set in a build that compiled the overlay out. Verified clippy `-D warnings` exit 0 on the edited (default) arm, fmt clean, 101 agent lib tests pass. NOT gated by a test — the repo has no log-capture harness and adding a dep for one line is disproportionate. The operator's half stands: nothing enables `agent/forecast-audit-tick`, so the flag remains unreachable — wire it or delete the documented option.
- 2026-08-15 (orchestrator): **#92 added (OPEN)** — the `--no-default-features --features fixtures` build that `crates/ui/Cargo.toml` documents *"for the gallery-only bin"* fails with 3 `E0432` errors: `agent` is a `live`-only dependency but `state.rs` (16 refs), `lab/activity.rs` (3) and `widgets/activity_tape.rs` (2) import it unconditionally. Nothing in CI, scripts, skills, README or runbooks builds this configuration. **Blocks #91**, whose fix lives in exactly this arm and cannot be compile-verified until it builds — so #91 is on hold rather than applied blind.
- 2026-08-15 (orchestrator): **#91 added (OPEN, LOW)** — `spawn_lab_run`'s non-`live` arm returns `Ok(RunSummary{empty})` where both siblings return `Err`. Logged WITH a downgrade: the audit's scarier claim — the same shape on a runtime branch inside the SHIPPED cockpit — is **false**; `rt_handle` is a non-`Option` field and the sole production caller passes `Some(&…)` unconditionally, so that arm is unreachable. No build compiles the cfg arm either (`live` is default; `--features fixtures` is additive — confirmed against cargo's resolver). What earns the entry is dated: the stub's rationale landed 2026-05-24 and `live` became default 2026-05-25, so the comment has defended a one-day-old build configuration for three months.
- 2026-08-15 (orchestrator): **#80 FULLY CLOSED — both halves.** The forward paper loop (`crates/agent/src/runtime.rs`, the path that executes the operator's ACTUAL plan, not merely the ranking bake-off) routed its short legs around the engine exactly as the ranking side had; it is now sized via `plan_open_short`, stepped inline through `engine.step`, and accounted in the ONE existing per-fill block. Gated by a new 4-assertion e2e in **`crates/agent`** — structurally required, since the existing `backtest` gate cannot observe that crate's loop. Orchestrator-verified: gate is NON-VACUOUS (`assert_actually_shorted`, floor `MIN_SHORT_LEG_FILLS = 10`, measured 56 short legs of 85 fills; slippage rate asserted `> 0` so zero-vs-zero cannot pass as parity); 4 passed / 0 failed; FROZEN AD-1 files byte-untouched; anchors 119/119 both sides. Measured blast radius on the `_ls` arm: slippage +41.4 %, total friction +10.75 %, aggregate rate now exactly 2 bps; long-only control byte-identical to the last digit. **Structural result: `try_open_short` and `try_cover_short` — the two self-accounting helpers at the heart of #80 — now have ZERO production call sites workspace-wide** (`#[cfg(test)]` opens at `short_exec.rs:434`; every remaining hit is past it). The seam is gone from production, not merely patched at two sites.
- 2026-08-15 (orchestrator): **#90 added (OPEN, deliberately scoped out)** — `short_exec::check_and_liquidate` force-covers at the raw mark with fee only, emits NO `Fill`, and is therefore invisible to BOTH friction-parity gates. Symmetric (one caller per side), so not a repeat of #80's asymmetry — but it is the #74 channel shape: the gates are sound about what they see, and this path never reaches their input. Left unfixed on purpose — engine-routing it changes WHAT is liquidated and WHEN (slippage moves the cover price, which moves the equity that triggered the liquidation), a feedback loop that needs its own decision. Three options costed in the entry.
- 2026-08-15 (orchestrator): **#89 added (OPEN)** — `PaperEngine`'s seed is provably inert: `rng: ChaCha20Rng` is seeded in the constructor and NEVER read (`grep 'self.rng'` → nothing), with `#[allow(dead_code)]` directly above it silencing the compiler that had already found it. Anchor-safe by construction — nothing in the fill path is stochastic — which is exactly why 119 locked anchors and a 14-story review never surfaced it: a parameter that does nothing looks identical to determinism. Latent trap: any future seed-varying test would be tautological. Bears on 1-25's queued FILL_SEED domain-separation rider, which buys nothing until the value is actually consumed.
- 2026-08-15 (orchestrator): **#88 added (OPEN)** — AD-10's rendered-pixel evidence base is macOS-only: 32 test files are `#![cfg(target_os="macos")]` and 31 carry pixel assertions, so the Linux and Windows CI legs go green having executed ZERO of them. ADR-0057 correctly pins BYTE-COMPARE baselines to one box; it does not cover the STRUCTURAL pixel-count harnesses, which compare a render against itself and could run anywhere. A 3-OS matrix implying coverage it does not have — the same illusion as a gate that cannot fail, one layer up.
- 2026-08-15 (orchestrator): **#87 added (OPEN)** — a documented operator opt-in (`[strategies.tcn_overlay_momentum] enabled = true`) cannot work in any build: the registration is `#[cfg(feature="forecast-audit-tick")]`, nothing enables that feature, and the off-arm is a silent `let _ = ledger;`. #81's shape with a CONFIG FILE as the operator surface. Framing corrected on verification — the stanza is NOT currently set in `config/agent.toml`, so the defect is a documented control that cannot work, not one currently ignored.
- 2026-08-15 (orchestrator): **24-FEATURE CFG AUDIT complete** (`docs/dev-notes/feature-reachability-audit-2026-08-15.md`). The detector generalised from #81 reproduced BOTH known positives from resolver output before reading any source, and its enablement column agrees with the reachability map on all 24 rows. `cargo metadata` confirms **24** features, not the 7 an earlier hand-count of manifests found. Key refinement: `backtest/realdata` is NOT uniform — 10 of its 13 `cfg(not(...))` sites `bail!` with the rebuild command, 1 logs, and only `resolve_dvol_override` returns a bare `None`, so #81's severity is concentrated in ONE site rather than thirteen.
- 2026-08-15 (orchestrator): **#86 added (OPEN, governance)** — AD-18's ADR gate is ONE-WAY: `adr_registry_check.py` checks every decision FILE has a registry ROW, never the converse. 86 files vs 87 rows; `0079-*.md` does not exist while row 0079 sits `accepted` and detailed — and it is load-bearing: `vol_estimator.rs`, `strategy/lib.rs` and **ADR-0078's "Consumes: ADR-0079"** all cite reasoning that was never written. Fix (~5 lines) deliberately NOT applied: it turns a green pre-commit/CI gate red until the operator either writes the decision or deletes the row. Bidirectional invariant, unidirectional gate.
- 2026-08-15 (orchestrator): **#85 added (OPEN)** — two account-level loss stops (`daily_loss_stop_pct` −5.0, `max_drawdown_stop_pct` −15.0) are declared in `RiskConfig`, set in `config/agent.toml` AND `agent.toml.soak-fast`, documented in the soak runbook as kill-switch trip conditions, and called "the portfolio-level floor" by ADR-0010 — with **ZERO read sites workspace-wide**. #79's shape applied to a risk limit. Paper/sim only so no live money is exposed, but a declared-and-absent safety control is dangerous precisely because it looks present.
- 2026-08-15 (orchestrator): **#81's arm-count fix was INCOMPLETE — my own gap, now closed.** `arm_runs_in_this_build()` handled only `v0.macro_riskon`, so the cockpit declared 19 arms while 18 ran: `v0.dvol_regime` has the identical build-time impossibility behind `backtest/realdata`, which I documented in #81's extension and then failed to connect to the count. Added `dvol_arm_compiled()`, extended the predicate, and fixed a double-subtraction my own change would have introduced in `advisor_field_arm_count_for` (the build filter and the per-coin filter were both charging for the same arm). The test is now data-driven over the gated pair and derives the expected count instead of asserting the literal `19` — a hard-coded expectation is how the count drifted from the field in the first place. `cargo test -p ui --lib leaderboard` 55/55.
- 2026-08-15 (orchestrator): **#84 added + FIXED (CI)** — the CI anchors gate COULD NOT FAIL. `bash scripts/verify_anchors.sh | tail -1` ran under GitHub's default `bash -e {0}`, which does NOT set `pipefail`, so the step exited with *tail's* status (always 0). AD-2's only remote enforcement was inert from CI activation until now. Demonstrated: `bash -e -c 'false | tail -1'` -> 0 vs `bash -eo pipefail` -> 1. Fixed with `shell: bash` (Actions then uses `-eo pipefail`) plus a load-bearing comment. `spec_lint.py` on the next line is unpiped and DID gate. The local pre-commit hook sets pipefail, which is why the burn-down's local runs were meaningful. Found by the claims-vs-reality ledger.
- 2026-08-15 (orchestrator): **#82 SCOPE CORRECTED — my own overstatement.** The entry claimed the operator's advisor evaluates the short slate; `default_short_field()` has ZERO production callers (only an #[ignore]d P2 harness), so the cockpit never runs those arms. All measurements stand; the damaged record is the RESEARCH lane feeding the era-qualified thesis, not the operator's screen. I asserted a reachability claim without tracing the caller graph — the declared-not-executed error, committed while documenting it.
- 2026-08-15 (orchestrator): **#83 added (OPEN — in the FROZEN gate)** — first audit of `robustness.rs`/`rank.rs`/`bootstrap.rs`, the 664 production lines that decide every verdict and which the burn-down never opened because AD-1 freezes them. The gate **fails OPEN**: `compute_robustness_distribution` returns `None` on a degenerate curve, empty returns, OR the NaN/non-finite guard firing; `compute_robustness_flag` maps `None => Skipped`; and `is_eligible` is a DENY-list (`!= Some(Fragile)`), so `Skipped` is crown-eligible. Failure and deliberate-non-execution share one permissive flag. Live reachability UNPROVEN and stated as such; the design defect stands regardless. Needs an AD-1 decision, not a patch. **Verified CLEAN in the same audit**: the band arithmetic, the weakest-link/all-pass structure, the drawdown UNITS (a documented fraction vs the 0.70 threshold — no repeat of the 100x bugs), and the NaN guard itself, which is correct and well-reasoned.
- 2026-08-15 (orchestrator): **#81 EXTENDED — the defect has a TWIN, `backtest/realdata`**, found by generalizing #81 into a detector over all seven cfg-gated capabilities. Same signature (enabled by nothing, no `default` stanza); it gates `dvol_data`/`basis_data`/`funding_data` and `resolve_dvol_override`, whose non-realdata variant returns `None` unconditionally. Verified: `ui`'s entire `[features]` section mentions `backtest` ZERO times and the documented run commands pass nothing to it, so the shipped cockpit builds `backtest` with NO features. **Corrects #78's DVOL framing**: that arm's inertness is unconditional (loader not compiled), not the corpus-absence problem #78 described. Its drop-to-ABSENCE guard already landed, so it is correctly absent today.
- 2026-08-15 (orchestrator): **#82 added (OPEN)** — the advisor's entire SHORT SLATE never shorts on real data: 5 arms ranked as long/short take ZERO short legs on both windows; `v0.sma_cross_ls` ratchets 181 buys against 1 sell to ~11-16x leverage and negative equity because the side-blind cap (#71) refuses its EXITS while its ENTRIES pass; three others are alternation-locked and can never reach flat; `v0.always_short` takes zero fills. #71's consequence on the product surface. Found while fixing #80; the new `[SHORT-CENSUS]` line makes it permanently visible.
- 2026-08-15 (orchestrator): **#80 FIXED** — short legs now route through `PaperEngine::step` (shape A), chosen after verifying the engine can already represent a short open/cover (montecarlo does it; `Order::new` is side-blind; `apply_sell(short_enabled)` exists per ADR-0068 D1). One friction site and one accounting site now serve both leg families, so a future friction change cannot land on one path only. New parity gate asserts friction-per-notional matches long-only vs short-enabled; restoring the old branches turns 3 of 4 gates RED. Un-rounded advisor fills 20/194 -> 0/194; decisions preserved (194 fills both sides). Anchor-neutrality proven beyond the gate by RE-DERIVING `btc-2023-1m-sma-cross` to an exact match plus a byte-identical A/B on all four legacy CLI anchors.
- 2026-08-14 (orchestrator): **#81 HONESTY HALF FIXED** — the macro arm is now dropped to ABSENCE when its regime series is unavailable (mirroring the DVOL guard in the same function), routed through a new `arm_runs_in_this_build()` predicate that the cockpit's arm count also reads, so the advisor honestly reports 19 arms instead of ranking one that never ran. RED-proven. Also: the forward loop now bails instead of substituting AlwaysLongStrategy; the 3-AND rule finally has a binding test; S3 is a real prefix-invariance causality falsifier; a span-coverage bound was added; and the tautological T-CAL tests now call production, binding `expected_bars_for_range` which had ZERO test call sites. **The capability half (enabling `backtest/yahoo`) is deliberately NOT flipped and is BLOCKED on the emission-cadence defect** — enabling it first would yield a working-but-WRONG arm rather than an inert one.
- 2026-08-14 (orchestrator): **#80 anchor impact MEASURED — NO** (was recorded unmeasured). The `short_exec` engine bypass is confined to `sma_composed_run.rs` (advisor lane, `write_report=false`); `montecarlo.rs` — which writes the anchored theta-surfaces incl. the MN short-traffic family — has ZERO `short_exec` calls (its three mentions are comments + one assert string) and routes shorts through `Order::new`/`engine.step`; `threshold_sweep.rs` has none. Fixing #80 cannot move an anchored number. Blocker cleared.
- 2026-08-14 (orchestrator): **#81 added (OPEN — most severe product defect of the burn-down)** — `macro_regime.rs` is `#![cfg(feature="yahoo")]` on **backtest's** feature, `backtest` has no `default` stanza, and NOTHING in the workspace enables `backtest/yahoo` (`ui` enables `data/yahoo`, a different crate's feature; Cargo does not unify across crates). So the macro loader is **never compiled** and `v0.macro_riskon` has run 100% cash in every build since it shipped — unfixable by fetching data. Compounding: the forward paper loop substitutes `AlwaysLongStrategy` under the same label, so the two degradations of one arm are OPPOSITE (bake-off cash, forward long), justified by a false equivalence in a code comment; and the P2 multi-corpus rerun counted the arm as evaluated on the 2021-22 bear corpus where it also ran cash. Not a crown risk (flat curve fails the FRAGILE band, verified). Story-3-16 review (burn-down 14/14).
- 2026-08-12 (orchestrator): **#80 added (OPEN)** — short legs bypass `PaperEngine::step` via `short_exec` + `continue`, so they pay the taker fee but NOT slippage, lot-rounding or the fill-price model. The bake-off ranks long arms (which pay all of it) against short arms (which do not), so every short-enabled arm is flattered in the comparison that decides what the operator sees. Witnessed: `v0.sma_cross_ls` leaves 20 of 194 fills un-rounded on the advisor path. Thesis unaffected (the crowned arm is passive); anchor impact UNMEASURED — check whether any anchored lane runs short legs before fixing.
- 2026-08-12 (orchestrator): **#79 FIXED** — 13 arms threaded through `run_scenario`, the filter applied at all 3 engine construction sites (incl. the inline vote-arm engine that would have left a third of the field inert), and the mis-named "ADVISOR-PATH GATE" re-pointed at production with traded/mechanism/effect witnesses. Both links mutation-proven RED. Blast radius on the real corpus: typical arms < 0.1% of terminal equity. Anchors 119/119 both sides. Nine other runners share the gap but are provably inert (callers pass None) — left alone, recorded as a latent repeat.
- 2026-08-12 (orchestrator): **#79 added (OPEN, CRITICAL — product)** — €200 lot realism (PRD §13 Q5, shipped `de571de` 2026-08-04) is INERT on the advisor path: `advisor_default()` sets `venue_filter: Some(...)` into `ScenarioConfig`, `run_scenario` threads it for only 1 of ~15 arms, and `run`/`run_with_strategy` never call `with_venue_filter_mode` at all — which has **zero** production call sites workspace-wide. The gate named "ADVISOR-PATH GATE" builds its own engine and asserts a constructor value; it contains zero calls to `run_bakeoff`/`run_scenario`/`sma_composed_run`. All ~14 bake-off arms fill without lot-size/min-notional realism while every artifact says it is on. Crowns/thesis unaffected (the advisor gate resamples returns); the harm is that recommended positions may not be placeable at €200. Found through the story-3-15 review, owned elsewhere.
- 2026-08-12 (orchestrator): **#78 added (OPEN)** — "graceful degradation" keeps the DVOL probe arm in the ranked field under its real label while running 100% cash (the corpus is gitignored, so this is the DEFAULT state of every fresh clone), and five code comments call that a "buy-and-hold proxy" — false, because the same arm's warm-up path never emits a Buy. Propagated to story 3-16's macro arm by explicit citation, which makes it a class. Not a crown risk (verified: the flat curve is never `is_eligible`); the harm is presentational, on an honesty-first product. Story-3-15 review (burn-down 13/14).
- 2026-08-11 (orchestrator): **#76 added (OPEN, CRITICAL)** — the basis⊥funding RESIDUAL arm ranks the basis axis inverted vs its own spec (longs the HIGHEST basis; rank 1 = lowest basis, and `top_k_long` takes the highest residual). With #75 this means **no anchored MN surface tested the basis in its documented direction**, so "the residual carries no orthogonal alpha" cannot be read off #116-#119 at all. Every residual test asserts difference, never direction. Story-1-21 review; fix + re-run → 1-25.
- 2026-08-11 (orchestrator): **#75 added (OPEN, CRITICAL)** — `run_path` overwrites the pre-injected SCORE map with the ACCRUAL map (one `funding_map` field, two meanings), so the market-neutral BASIS arm silently ran the FUNDING score. Anchors #108-#111 are duplicate funding runs; the k2 kill-criterion and the "domain CLOSED with finality" claim rest on the artifact. Confirmed with a control (`mn-basisperp`, whose basis rides a different field, differs in every number while `mn-basis` differs in none). Story-1-21 review (burn-down 10/14). Fix + re-run → 1-25. Live records corrected same pass; the anchored bodies cannot be.
- 2026-08-11 (orchestrator): **#74 added+FIXED** — the AD-16 day-1 divergence gate for the basis arm was vacuous because the signal was injected through `funding_override`, which is also the accrual channel (~60× the test epsilon); the suite stayed green with the signal returning constant zero. Derived independently by two review layers, verified at source. Story-1-20 review (burn-down 9/14). Spawned the ninth mandatory probe (**channel**) in the review playbook. Also this pass: anchors #100-#107 routed to 1-25; ADR-0086's basis publication-lag justification corrected (declared 0, grounded 3_600_000 — ruling deferred to 1-25 as anchor-impacting).
- 2026-05-25 (orchestrator): file created. Backfilled #54–#63 from `git log` + inline `Bug #N` comments.
- 2026-05-25 (orchestrator): #64 added — progress bar short-run starvation fix.
- 2026-05-26 (orchestrator): #65 added — vol_killswitch_overlay no-op discovered by Wave 1 overlay-e2e test; 2 tests `#[ignore]`-gated pending source fix.
- 2026-05-26 (analyst): #65 updated — analyst brief authored at [`spec/vol-killswitch-overlay-noop-fix v0.1.0`](../archive/pre-bmad-spec/v1/vol-killswitch-overlay-noop-fix/feature.md). P0 safety; trace row `REQ-VOL-KILLSWITCH-NOOP-FIX-001` at `proposed`; sibling of shipped `v3-volatility-forecaster-noop-fix v0.1.0` 2026-05-22. Status flipped `open` → `open (analyst brief authored)`.
- 2026-05-26 (developer): #65 FIXED — Q4=(p3) "Both" fix shipped. A.1: lookback_minutes 60→5 + flat warmup prevents GARCH early-kill. A.2: overlay filter broadened to basket-wide Hold. A.3: #[ignore] removed; 4/4 tests green. Hygiene gate 2/2 pass.
- 2026-08-04 (orchestrator): #70 added+FIXED (coverage gate compared coarse-vs-raw units), #71 added (OPEN — exposure cap side-blind, blocks de-risking; the dev softened a fixture around it), #72 added (OPEN — cosmetic 1h ladder made funding accrual horizon-blind; carry-coarse anchors measured a throttled mechanism; errata issued same day). All three from the story-1-18 review.
- 2026-08-04 (orchestrator): #69 added (OPEN) — portfolio_exposure_cap inert engine-wide (the #68 lineage at the risk-limit layer); D-TSM.2 premise false; TS surfaces ran ~2× documented gross; enforce-or-delete + thesis re-affirmation ride 1-25. #67 blast radius extended to anchors #90/#91.
- 2026-08-03 (orchestrator): #68 added (OPEN) — the θ-grids' drift/hold-band swept axis is inert (the #65 class, one layer up); implement-or-drop rides 1-25; #67 blast radius extended to anchor #87.
- 2026-07-31 (orchestrator): #67 added (OPEN) — cross-symbol fill mispricing in the research-harness lanes; anchored C2/C3 evidence is execution-artifact noise; advisor gate proven unaffected; fix+re-lock = story 1-25 (program with 1-24).
- 2026-07-27 (orchestrator): #66 added+FIXED — ui real-data guard tests vacuous since day 1 (cwd-relative corpus root, any-Err→skip); revival caught 3 latent production bugs (CSV-name test bug, scenario-name collision/shadowing, unindented-frontmatter Compare skip). Story 1-10 code-review pass; all gates re-verified (anchors 119/119, spec-lint 0, clippy 0, AC5 4369-point round-trip).

**Observed again 2026-09-24** while gating story 3-21, and this is what the open decision
costs in practice: `audit_aggregator_handles_10k_event_storm` fails **1 run in 5** unloaded on
macOS, always the same way — `start_label=152..240 + ticks=0 = 0.03..0.05` coverage. `ticks=0`
means the whole 5 000-event burst landed inside ONE window, so no `Tick` ever fired and the
window's real count went out with the bus. The Start label alone caught the first ~200 events.
The test is therefore a coin-flip on whether the burst happens to straddle a boundary, and a
`cargo test -p ui` run aborts at it before reaching the other ~85 test binaries. Attribution
for 3-21: not that change — the same commit's earlier full run passed this test, and 4 of 5
runs pass at the same tree. It stays OPEN pending the product-side ruling.

### `#100` — the cockpit renders report bodies from disk, and those carry glyphs the embedded face cannot draw
**Status**: OPEN — measured, scoped, not fixed. Found 2026-09-16 while wiring ADR-0093.
Anchor-impacting: **no** (reads the corpus, never writes it).

ADR-0093 makes every glyph the cockpit draws come from a face this repo ships — for text
written in `crates/ui/src`. `crates/ui/tests/embedded_font_contract.rs` enforces exactly that,
by scanning string literals. But the Reports screen (`crates/ui/src/screens/reports.rs:324`)
and the `viewer` binary (`crates/ui/src/bin/viewer.rs:111`) render `body_markdown` LOADED
FROM `evidence/`, and no static scan can see it.

**Measured** over the 321 committed evidence documents: **11 glyphs Inter cannot draw, in 21
files**.

| glyph | count | files | source |
|---|---|---|---|
| `▁`..`█` (block elements) | 480 | 4 | `reports::render::equity_curve` -> `sparkline::encode` |
| `∧ ∈ ⊥ ≡ ≫ ∪ ≪` | 26 | ~10 | set / logic notation in report prose |
| `ⓘ` | 2 | 1 | a quoted Lab notice |
| `✅` | 1 | 1 | an ERRATA marker |

cosmic-text falls back PER GLYPH, so on that screen those characters are drawn by whatever
OS font has them — the exact instability ADR-0093 removes everywhere else.

**Why nothing is red.** No byte-exact baseline is a Reports screen, and the ui's own report
fixture body (`reports_populated_curve_render.rs:194`) is plain ASCII plus an em dash, which
Inter has. So no gate depends on the fallback today. A future Reports-screen baseline would
drift with the OS until this is fixed — which is the trap #96 taught: the reservation is real
but unenforced.

**Fix direction (not taken here).** Prefer DRAWING the equity sparkline as a widget on that
screen instead of rendering typed block characters — it removes 480 of the 483 occurrences and
is the honest fix, since a sparkline is a picture. The residual notation glyphs are report
prose; either accept them as documented fallback or normalise the report vocabulary. Embedding
a second, block-capable face is the expensive option and buys the least.

**Moral**: a contract enforced by scanning source covers what the source says, not what the
program loads. State the scope in the test, or the next reader will believe the stronger claim.

### `#101` — a negative control that ADR-0085 had already falsified, red since, and never diagnosed
**Status**: FIXED 2026-09-16. Found while re-baselining for ADR-0093; attributed by stash A/B.
Anchor-impacting: **no** (cockpit surface).

`leaderboard_long_only_is_the_negative_control_for_shorts` asserted two things about the
long-only field: that it paints strictly LESS warn-amber than the short field, and that it
paints **`< 40`** amber px at all — "no short field, so no unbounded-loss disclaimer".

The second claim stopped being true when **ADR-0085** added the crown-credibility band to
the recommendation banner. That band is `WARN_500`-amber and paints in EVERY frame whose
crown does not clear DSR — the modal case for these fixtures — so the long-only control
carries amber that has nothing to do with short selling.

**Measured, not inferred.** With this session's changes stashed (plain HEAD):

```
long = short = 1611 amber px   ->  assertion 1 fails as a tautology
```

Both assertions were therefore unreachable-as-intended, and the file has been red since
ADR-0085 landed. The 2026-07-27 drift note counted this file's one failure among the "62
baseline comparisons" and attributed it to font drift; it was not font drift.

**Why it surfaced now.** The file rendered a 2400-px frame, which since ADR-0092 (the
scorecard moved to the top of the pane, +358 px) no longer reached the bottom-anchored
short-field disclaimer — `iced_test::screenshot` CLIPS to the viewport. Raising the frame
to 3000 px and asserting the pane fits captured the disclaimer again, which made the short
frame paint strictly more amber than the control, which moved the failure from assertion 1
to assertion 2 and finally made the real defect legible.

**Fix.** The discriminator is the DELTA, not an absolute: both frames legitimately carry
ADR-0085's band, and only the short field draws the unbounded-loss disclaimer on top of it.
The absolute `< 40` claim is gone, with the measurement and its reason recorded at the
assertion.

**Moral**: a negative control asserts what a feature does NOT paint. Every later feature
that paints in the same channel silently weakens it — and when the control finally fails,
the cheapest story ("font drift", "flaky pixels") is the one most likely to be believed.
Attribute a red before you re-baseline it away: `git stash` + run at HEAD costs one minute.

### `#102` — lab-state persistence was shipped, unit-tested, and never wired: the cockpit neither restores nor saves it
**Status**: **FIXED 2026-09-24** (ADR-0094, story 3-21). Found the same day while scoping 3-21.
Anchor-impacting: **no** (no evidence path; the file lives in `$XDG_CONFIG_HOME`).

Story 3-21's inventory named the defect as `lab::persistence::decode` silently cold-starting on
a version mismatch or a parse error. Checking the blast radius before fixing it turned up a
larger one: **nothing calls the feature at all.**

```
# the read path
crates/ui/src/state.rs:1529   pub fn boot(state_path_override: Option<&Path>) -> Self   <- restores
  callers: crates/ui/src/state.rs:5736, :5862                                           (its own unit tests)

# the write path
crates/ui/src/lab/persistence.rs:307   PersistenceDebouncer
                                :329   flush_if_due
                                :341   force_flush
  callers (scripts/callers.sh, CodeGraph ∪ grep): persistence.rs:450, :474, :475, :492   (its own unit tests)
```

Both shipped binaries construct the cockpit with `Cockpit::new()` — `bin/cockpit.rs:174` and
`bin/cockpit_live.rs:705` — never `Cockpit::boot(..)`. `cockpit_live` has a clean shutdown
sequence (`:1012-1028`) that would be the natural `force_flush` site and does not call it.

**Consequence.** The Lab selection (strategy, pair, date range, compare set, training-panel
state) is written to disk never and read from disk never. Every launch is a cold start, and
`~/.config/trading/cockpit-lab-state.json` is not created by the shipped cockpit at all. The
500 ms debounce, the XDG path resolution, the schema-version field, the corruption fallback and
their nine unit tests are all exercised only by callers inside the module.

**Why nothing is red.** Every test calls the API directly — `boot(Some(&path))` in the unit
tests, `encode`/`decode` round-trips in-module. A test that constructs the thing it tests cannot
observe that the product does not construct it. This is the same shape as `#95` (declared at
nine sites, read at one) and `#81` (an arm whose loader never compiled): the unit is correct,
the wire is missing, and the suite only ever looks at the unit.

**How long.** Shipped `c654f31` (2026-05-17, ui-rethink-phase-a-lab Wave 2, T-D-17). Bug `#54`
(`799543a`) later changed the cold-start default from `v1.momentum × XRPUSDT` to
`v0.sma × BTCUSDT` — a fix written on the belief that the cold-start path was user-visible. It
was, but only because *every* start is a cold start. There is no CHANGELOG line for the feature.

**Fix direction.** Wire it, then fix the load path 3-21 actually asked about — in that order,
because a migrate-or-fail-loud contract on a load that never runs is ceremony. Wiring makes the
file real, which makes the silent-reset defect real too: today a failed load returns cold-start
defaults and the next flush *overwrites the user's file with them*, so the failure mode is data
loss, not data ignored. That is why the load path must distinguish "fresh" from "failed" before
the write path goes live, and must never let a file it could not read be overwritten.

**Fixed by** ADR-0094: `Cockpit::boot` replaces `Cockpit::new` in both binaries; the writer is
driven from `ui::state::update` (the seam both cockpits share) on an explicit five-message list;
`lab_state_path: Option<PathBuf>` is the single switch that arms it, and every fixture, gallery
and test constructor sets `None`, so a demo cockpit cannot write the operator's file. The load
path then got what 3-21 actually asked for: `decode` returns `Result<_, FailureReason>`, `restore`
returns the outcome alongside the state, and a file that cannot be used is renamed aside BEFORE
the load returns. `restore_or_default` was deleted — after the wiring its only callers were, once
again, its own tests.

**Moral**: "is it tested?" and "is it reachable from `main`?" are different questions, and the
green suite answers only the first. Ask `scripts/callers.sh` of any feature you are about to
extend — the extension inherits its reachability, not its test count.

### `#103` — a paused tokio clock silently kills every sqlx write under it, and the hazard was already written down in a different test's doc comment
**Status**: FIXED 2026-09-24 in `coinbase_outage_isolation.rs` (story 6-9 shakeout).
Anchor-impacting: **no**.

`t1414_v7_coinbase_outage_isolated` has been red on `windows-latest` since the 3-OS matrix
was activated, reading `got 0` where a `FeedReconnect` row was expected. It is not a slow
runner. The test called `tokio::time::pause()` before both ledger interactions, and:

- every sqlx pool acquire wraps itself in `tokio::time::timeout(acquire_timeout /* 30 s */, …)`
  (`sqlx-core-0.8.6/src/pool/inner.rs:248-251` → `rt/mod.rs:27`) — a deadline on the
  **virtual** clock;
- the SQLite work runs on a raw `std::thread::Builder` worker
  (`sqlx-sqlite-0.8.6/src/connection/worker.rs:106`), **not** `spawn_blocking`, so it never
  inhibits tokio's auto-advance (the only inhibitor is `runtime/blocking/schedule.rs:25`);
- so the moment the runtime parks mid-write, virtual time leaps to the next deadline —
  repeatedly — and the acquire times out after microseconds of real time;
- `spawn_venue_supervisor` swallows the resulting `PoolTimedOut` as a non-fatal `warn!`
  (`crates/agent/src/runtime.rs:2872-2880`), so the only symptom is a missing row.

**Measured** (temporary probes, since removed): 16 concurrent `feed_reconnect` writes under
`pause()` → **9 returned `PoolTimedOut` after 481 µs of real time**. And the 200-iteration
bounded poll added in 2026-09 to "give the writer a chance" burns **96 virtual seconds in
39 ms of real time** against that 30 s deadline — it cannot help, and past 30 s it is what
kills the write.

**The part worth keeping.** This exact hazard was already documented — in the doc comment of
a different test:

> `crates/ui/tests/training_poller_subscription.rs:10` — "`tokio::time::pause()` + `advance()`
> is incompatible with `sqlx`'s in-memory SQLite pool: the pool's connection-acquire timeout
> fires immediately when time is frozen, producing `pool timed out`."

Someone hit it, diagnosed it correctly, wrote it down where they were standing, and moved on.
The next test walked into it anyway and cost a red CI lane for two months. A hazard recorded
in one file's prose is not a repo-wide guard.

**Fix**: the clock is paused only across the MockFeed/watchdog phase; supervisors start on
the real clock, the `FeedReconnect` assertion moved ahead of the pause as a bounded
**real**-time poll, and `resume()` precedes cancel+drain. Checked mechanically and
independently: no `audit::` / `ledger.` / `journal::` / `query::` call now appears between
`pause()` (line 361) and `resume()` (line 483).

**Not proven**: that `windows-latest` specifically loses this race — that needs the lane to
go green, which is story 6-9's own gate. macOS was 30/30 under 42 CPU burners both before
and after, which is the same ceiling `#99`'s sibling diagnosis hit.

**Moral**: an invariant that lives in one file's doc comment will be re-discovered the
expensive way. If a rule is repo-wide, it belongs where the next person is looking — the bug
log, CLAUDE.md, or a test that fails when it is broken.

### `#104` — the secrets gate scans with `strings(1)` and swallows its absence, so on a box without binutils it passes having read nothing
**Status**: FIXED 2026-09-24 (story 6-9 shakeout). Anchor-impacting: **no**.

`scripts/check_no_secrets_in_llm_artifacts.sh` is the V9 gate: no substring of an API key may
appear in any artifact the LLM smoke run writes. It scans like this (`:146`, `:150`):

```bash
if strings -- "$file" 2>/dev/null | grep -i -q -F "$pat"; then
```

`strings(1)` is binutils. It ships with macOS (Xcode CLT) and every Linux runner, and it is
**absent from Git-for-Windows**. When it is missing the pipeline produces no output,
`grep -q` reports no match, and the file is declared clean — and `2>/dev/null` eats the
"command not found" that would otherwise have said so. A secrets gate that passes having
scanned zero bytes is the `#66` vacuous-test failure mode, on the one check whose whole
purpose is to be paranoid. GNU `find` is shadowed the same way by `C:\Windows\System32\find.exe`.

**Fix**: the scan itself is Rust now and runs on all three legs
(`crates/llm/tests/no_secrets_in_artifacts_test.rs`). The script stays the single source of
truth for the pattern list — the test PARSES `PATTERNS=(…)` and `SK_RE=` out of it at
runtime and refuses to run if the list shrank below 8 entries or `SK_RE` changed spelling,
so the two cannot drift silently. The Rust scan is a strict superset of the shell one: raw
bytes, so no `strings` 4-character floor and no `grep` line-splitting. On unix the script is
still executed verbatim as an additional assertion, keeping its own `find`/`grep`/`strings`
plumbing exercised. Three controls the shell gate never had now run every time: the matcher
detects both fixture keys and their upper-case forms, stays quiet on innocuous bytes, and the
directory WALK finds a planted key in a throwaway tree. Red-proofed by compiling out the unix
leg and planting a key: 4 hits, failing test.

**Coverage lost on Windows**: the script's `find`/`grep`/`strings` plumbing — never the
pattern list, never the artifacts. Strictly more is covered there than before, which was
nothing.

**Not proven**: that this vacuity is what made `t1926_no_secrets_in_artifacts` RED on
`windows-latest` — a vacuous gate passes, it does not fail, so the red has some other
proximate cause (`bash` resolution from `std::process::Command` is the likely one) that
could not be pinned without a Windows box. The vacuity is real and structural either way,
and the fix removes both.

**Also note**: nothing in `.github/workflows/` or `.githooks/` invokes this script — it runs
only from the test. The "fires standalone against CI artifacts" rationale in its own header
describes an intended use that was never wired.

**Moral**: `2>/dev/null` on the tool a check depends on converts "I cannot run" into "I found
nothing". If a gate's scanner can be missing, the gate must fail when it is — not report clean.

### `#105` — a corrupt or tampered pinned corpus degrades SILENTLY to unpinned live data, and the log says the window wasn't covered
**Status**: OPEN — found 2026-09-24 while scoping story 3-20 AC1 ("show the data revision").
Anchor-impacting: **no** (the bake-off writes no report body — `write_report: false`, ADR-0059).

`resolve_bakeoff_bars` tries the pinned corpus first and falls through to a live fetch:

```rust
// crates/backtest/src/bakeoff/mod.rs:541
Ok(_) | Err(_) => {
    tracing::info!(target: "bakeoff.resolve", ...,
        "pinned corpus does not cover the window — using dynamic fetch");
}
```

`Err(_)` here includes `RevisionError::FileMismatch` and `AggregateMismatch` — the errors
`data::revision::read_and_verify_revision_manifest` exists to raise. So a corpus whose bytes
no longer hash to its manifest is handled identically to one that simply lacks the requested
window: the run proceeds on `data/binance-dynamic`, which by design has **no `REVISION.toml`
at all** (`crates/data/src/dynamic_cache.rs:43` — "live data is not reproducible, ADR-0061 D5").
The `info!` that records it states something that is not true in the mismatch case.

The Lab path does this correctly for comparison: it compares against
`BINANCE_PINNED_REVISION_SHA` and hard-fails (`crates/ui/src/lab/runner.rs:757-773`).

**What the verify call actually buys today**: only that the corpus is self-consistent, never
that it is *the* pinned corpus. `bakeoff/mod.rs:424` computes the aggregate and discards it.

**Why it matters beyond tidiness.** The headline product path is a RELATIVE lookback
(`LeaderboardLookback`, 2 weeks … 4 years → `DateRange::Custom { now-Nd, now }`,
`crates/ui/src/leaderboard/state.rs:858-862`), and the pinned corpus spans 2023-01-01 …
2025-01-01. On today's date NO relative window is covered, so the dynamic path is not the
exception — it is the normal case, and the pinned-corpus verification never binds on it.

**Fix direction (not taken here)**: split the arm. A `RevisionError` mismatch is a loud
failure or at minimum a distinct, truthful log line and a flag the UI can surface; "window
not covered" stays the quiet fall-through. Surfacing WHICH corpus a run actually used is
story 3-20 AC1's subject and needs the provenance enum described there.

**Moral**: `Ok(_) | Err(_)` in one arm spends the error you went to the trouble of computing.
A verification whose failure is indistinguishable from a cache miss is not a verification.

### `#106` — an anchored report body embeds a MEASURED wall-clock time, so only a machine as fast as the operator's can reproduce it
**Status**: **FIXED 2026-09-24** (same day, one line). Found while scoping story 6-12.
Anchor-impacting: **no** — verified `ANCHORS PASS (119 / 119)` before AND after the change;
the fix renders the bytes the committed body already carries.

`report::sma` prints the run's own duration into the hashed body:

```rust
// crates/backtest/src/report/sma.rs:81
let body_elapsed = input.body_elapsed_override.unwrap_or(elapsed_secs);
// crates/backtest/src/report/sma.rs:129
| Wall-clock time      | {body_elapsed:.1}s              |
```

Every anchored SMA scenario in `main.rs` pins that number — `body_elapsed_override: Some(0.1)`
or `Some(0.2)` at `:441`, `:460`, `:479`, `:498`, `:517`, `:536`, `:555`, `:574`. **One emitter
does not**: `crates/backtest/src/bin/run_yahoo_sma.rs:249` passes `None`, under a comment that
was true when it was written and is false now —

> `// No elapsed override — this is a new scenario, not replicating an anchor.`

— because `btc-yahoo-2024-1d-sma-cross` **is** anchored (`evidence/anchors.toml`,
`sha256 = 076929bb63d9bec03ec83684b85ced818ee32c0b2da41140712ec1d01de6a1e0`). The committed
body reads `| Wall-clock time      | 0.0s              |`, which means the operator's release
run finished in under 0.05 s.

**Consequence**: any re-run slower than 50 ms renders `0.1s`, the body diverges, and the anchor
misses — for a reason that has nothing to do with the data, the engine, or correctness. A debug
profile, a cold CI runner, or a slower machine is enough. The anchor is reproducible only by
hardware at least as fast as the box that made it, which is the opposite of what an anchor is for.

**Why nothing is red**: `verify_anchors.sh` hashes the COMMITTED body and never re-runs
(bug-log #93), so it cannot observe this. Nobody has re-executed this scenario since it was
locked.

**Fix**: `body_elapsed_override: Some(0.0)` at `run_yahoo_sma.rs:249`. It renders `0.0s`, which
is byte-identical to the committed body, so **no re-lock is needed and 119/119 holds across the
change**. It is a one-line prerequisite for story 6-12, whose whole point is that a third party
can reproduce a figure — and today they cannot reproduce this one unless their machine is fast
enough.

**Fixed**: `body_elapsed_override: Some(0.0)` at `run_yahoo_sma.rs:249`, with the stale comment
replaced by one that states the scenario IS anchored and why the value is pinned. Anchors held
at 119/119 across the change, as predicted — nothing was re-emitted, so no committed body moved.

**Moral**: a hashed artifact must contain no measurement of the machine that produced it. The
seven siblings that pin the value knew this; the eighth was written as a new scenario and
inherited the hazard when it was anchored, with the comment still asserting it was exempt.

### `#107` — an anchored report body says two things the engine did not compute: a bar count that is off by one, and a money figure rounded through `f64`
**Status**: OPEN — found 2026-09-24 while proving story 6-12's reproduction.
Anchor-impacting: **yes, if fixed** — both live in the hashed body, so either fix re-locks
`btc-yahoo-2024-1d-sma-cross` and its seven `btc-2023-1m-*` siblings. **Do not fix casually.**

Reproducing `btc-yahoo-2024-1d-sma-cross` from a fresh corpus root produced a body
**byte-identical** to the committed one (SHA `076929bb…`, verified). Reproducibility is not the
problem. What the body *says* is.

The same run's stdout and its own report disagree:

| | stdout | hashed body |
|---|---|---|
| bars | `Bars loaded: 366`, `Bars replayed: 366` | `\| Bars replayed \| 367 \|` |
| final equity | `$104560.07 USDT` | `\| Final equity \| $104560.08 USDT \|` |

**A — "Bars replayed" is not the bars replayed.** `report::sma` fills that cell with
`bars = state.equity_curve.len()` (`crates/backtest/src/report/sma.rs:158`). The equity curve
carries an initial point — equity *before* the first bar — so it is always `bars + 1`. The
console's 366 is the honest count; the published figure is one larger, and has been in every
SMA report since the template was written.

**B — the money figure is rounded through `f64`, and it changes the cent.**
`crates/backtest/src/report/sma.rs:36`:

```rust
let final_f = f64::try_from(final_equity).unwrap_or(0.0);
```

The console formats the `Decimal` (`${final_equity:.2}` → `.07`); the body formats the `f64`
conversion (`${final_eq:.2}` → `.08`). Same source value, two different cents, and the one that
gets **hashed, anchored and shown to the user** is the `f64` one.

This is the shape AD-9 exists to forbid — "no `f64` in money math; `rust_decimal::Decimal` +
the `Money<C>` newtype only". The ledger obeys it; the *presentation* of the ledger's answer
does not, and presentation is what the anchor preserves. Note the `unwrap_or(0.0)`: a
conversion failure would publish `$0.00` as a final equity rather than fail.

**Why nothing is red.** `verify_anchors.sh` hashes committed bodies and never re-runs
(bug-log #93), so it cannot compare a body against the engine. Nobody had re-executed this
scenario since it was locked, and the stdout that disagrees scrolls past.

**Why it matters beyond a cent.** The console is what a human reads before deciding a run looks
right and locking it. If the terminal and the artifact disagree, the human validates one number
and anchors another.

**Blast radius, corrected 2026-09-24 after the first estimate was too small.** The initial
entry said "this anchor and its seven siblings". Cross-grep of the report emitters:

| observable | emitters | anchored bodies carrying the row |
|---|---|---|
| A — `equity_curve.len()` as "Bars replayed" | `sma.rs` only | **26** |
| B — money through `f64::try_from` | `sma`, `momentum`, `pairs`, `regime_dispatcher`, `tcn_overlay` (all five) | **80** |

A changes all 26 deterministically. B changes only the bodies where f64 rounding actually
crosses a cent boundary — **measured, that is rarer than feared**: re-emitting the four
synthetic `btc-2023-1m-*` scenarios with the money formatted from `Decimal` produced output
**identical** to the f64 path. The Yahoo anchor is where the two diverge.

**BLOCKED: #107 cannot be fixed by re-locking today, because of `#93`.** Measured 2026-09-24,
at clean `HEAD` with no local modification (`git status` clean, verified):

| `btc-2023-1m-sma-cross` | committed body | fresh run at HEAD |
|---|---|---|
| Bars replayed | 525601 | **17544** |
| Final equity | $47290.03 | **$107381.95** |
| Total fees | $33435.478506 | **$1849.150109** |

525600 is the number of minutes in a 365-day year; 17544 is 731 × 24, the hours in two years.
So the scenario named **`1m`** now generates **hourly** bars over a **two-year** span. The same
divergence appeared in all four `btc-2023-1m-*` scenarios.

This is `#93` — "the code stopped reproducing the frozen evidence, and 119/119 stayed green" —
observed at full scale and in plainer units than that entry's hash comparison. **You cannot
re-lock an anchor whose scenario no longer reproduces its own recorded body for unrelated
reasons**: the re-emission would bake today's computation in and silently convert a tracked
drift into the new truth, which is exactly what D6.b's "don't silently mutate historical
evidence" forbids.

**Fix direction.** Both fixes change hashed bytes and therefore belong in story **1-26**'s
regeneration — which already owns re-emitting anchored surfaces, already owns `--out-dir`, and
must resolve `#93` first because nothing else can. Rename the row to what it is
(`Equity points`) or subtract the initial point; and format the `Decimal` directly instead of
converting. Until 1-26 runs, the numbers are what they are and this entry is the record.

**Moral**: byte-immutability preserves a claim, it does not audit it. An anchor can hold a
figure that was never true with perfect fidelity for as long as nobody re-runs the thing that
produced it.

### `#108` — the re-lock's "new namespace" is the one thing ADR-0038 § D6.b rejected by name, and its stated reason is live
**Status**: RESOLVED 2026-09-25 — operator ruled **outcome (A): follow D6.b as ratified**, and it
is executed. The 34 bodies are re-emitted in place under their existing namespaces; a D6.c
amendment was declined in favour of one corpus. `anchors.toml`: 119 rows before and after, none
added, none deleted, **34 SHAs changed, 85 byte-identical, 0 namespaces changed**, and the changed
set *is* the inventory — confirmed from both sides, since the new bodies in place made the gate
report exactly 34 FAIL / 85 PASS before a single SHA was touched. **ANCHORS PASS (119 / 119)**.
5-step record: [`docs/dev-notes/1-26-d6b-re-emission-2026-09-25.md`](1-26-d6b-re-emission-2026-09-25.md).
The stale-bodies objection is answered structurally: each re-emitted row keeps its superseded
numbers verbatim, with the old SHA, directly beneath the correction.

Story 1-26 AC3: *"Regeneration goes to a NEW namespace per ADR-0038/0045 § D6; old rows stay
byte-frozen."* It cites D6 as its authority. D6.b — the *wiring-bug-fix re-emission protocol*,
which is precisely the protocol for the situation 1-26 is in — says the opposite in step 4:

> the new SHAs land in `evidence/anchors.toml` **in-place under the existing namespaces**
> (Q2=(a) default — never bifurcate the namespace; never silently delete a row)

and lists **namespace bifurcation** under "Not in scope of this protocol", with the reason:

> a `*-postfix` namespace was rejected at Q2 — bifurcation invites future readers to consume
> stale bodies

**That reason is not hypothetical here.** The 34 old rows stay anchored and gate-verified under
`evidence/v1/**/reports/`, and this re-lock's errata shows their tail-drawdown figures to be
artefacts of bug-log #94: the old corpus carries cells at **100.00 %** tail drawdown where the
clean engine's corpus-wide median is **26.70 %**, improved in **156 of 156** cells. A reader who
greps the anchored corpus finds the artefact; the correction is one directory away and nothing
in the anchored body points to it.

**Why it was not caught earlier.** AC3 cites "ADR-0038/0045 § D6" — and plain § D6 *is*
compatible with a new namespace, because it only says existing anchors stay byte-identical.
D6.b is the 2026-05-22 amendment that narrows it, and it sits under a heading the citation does
not name. The citation is not wrong about D6; it is silent about D6.b.

**The mechanism for changing it already exists** and D6.b names it: *"If the protocol itself
needs revision … the revision lands as **D6.c** (additive amendment subsection, not in-place
mutation of D6.b)."* So the two coherent outcomes are (A) re-emit in place under D6.b with the
architect sign-off and negative invariant its step 5 requires, or (B) ratify D6.c permitting a
re-lock namespace **with a mandatory back-pointer from every superseded row**, which is the only
form that answers the stale-bodies objection.

Full argument, with the reproducibility evidence proving the bodies are anchor-grade either way:
[`evidence/v2/harness-relock/ERRATA.md` § 6b](../../evidence/v2/harness-relock/ERRATA.md).

### `#109` — an AC asks for pins to be re-derived from surfaces that do not contain them, and the file it targets says so in a comment
**Status**: OPEN — measured 2026-09-25, then PARTLY CORRECTED the same evening. AC7 is still
unsatisfiable as written and still needs its amendment; but the inference I drew from the
measurement was **wrong** and is retracted below.
Anchor-impacting: **yes** — see `#111`. (This entry originally said "no".)

> **RETRACTION 2026-09-25.** This entry concluded: *"the re-lock moved all 34 inventory surfaces and
> moved these four not at all, which is the confirming measurement for the comment's claim"* — the
> claim being that the two movements have different causes. The measurement is right; **the
> inference is wrong.** A bisect that evening put the cause of all four at **`11acd126`
> (2026-08-16) — the `#67` fix**, measured one commit either side (`b3332d35` → `0f6f6eb8…` GOOD,
> `11acd126` → `b655e5e7…` BAD). The four did not move during the 2026-09-25 regeneration because
> they had **already** moved on 2026-08-16. Same cause as the 34 surfaces, different date.
>
> The reasoning error is worth more than the correction: the exclusion rested on "these four do not
> go through `run_path`". True — and irrelevant. `#67`'s fix landed in `engine.rs` and `paper.rs`,
> the engine **below** every lane. Lane-scoping cannot bound a defect fixed beneath the lanes.
> Full write-up: `#111`.

Story 1-26 AC7 requires the four `#[ignore]`d drift gates in `crates/backtest/tests/determinism.rs`
to have their pins *"re-derived from the regenerated surfaces"* and the attribute removed in the
same commit. That is not possible as written. The four gates cover `top10-2023-1h-momentum`,
`top10-2024-h1-momentum`, `top10-2023-fy-tcn-overlay` and `top10-2024-fy-tcn-overlay` — **none of
which is among the 34 regenerated surfaces.** The 34 are all `*-theta-surface-*`.

`determinism.rs:722-731` already says why, and said it first:

> these four cover `scenarios/momentum.rs` and `scenarios/tcn_overlay.rs` … `run_path` — the lane
> D1 rewrote, and the one behind all 34 of the 1-26 inventory anchors — is not on their path. So
> 1-26 must NOT conflate the two sources of movement … Re-deriving one set of pins says nothing
> about the other.

**Measured after the re-lock at `2d63ddef`** — all four still RED, and every produced hash is
*unchanged* from the 2026-08-22 and 2026-08-23 measurements (`top10-2023-1h-momentum` is still
`b655e5e7…` exactly). The re-lock moved all 34 inventory surfaces and moved these four not at all,
which is the confirming measurement for the comment's claim.

So AC7 has three parts and they cannot all hold: *re-derive from the regenerated surfaces*
(impossible — not in them), *remove the `#[ignore]`* (would put CI permanently red on a defect
this story does not fix), *do not re-baseline* (correct, and bug-log #77's exact failure mode if
violated). The story's own Task note reaches the opposite conclusion from AC7 and is the later,
better-informed text.

**What the drift actually wants.** The same hash across three measurements spanning the ADR-0089
D1 sizer wiring and this entire re-lock means the divergence is deterministic and reproducible,
and a bisect against `83378c5` already places it *before* the #67/#71/#75/#76 fixes. It is a
bisectable defect with its own cause, and it wants its own story — not a line in someone else's
errata.

### `#110` — the re-lock re-ran the arithmetic through the same renderer, so every prose rider survived into a third generation of anchored bodies
**Status**: RESOLVED 2026-09-25 — operator ruled "fix the renderer and re-emit", and it is done.
The 12 MN surfaces were re-run through a corrected renderer and re-emitted in place. Not a D6.b
case (D6.b covers bodies reflecting a *wiring bug*; these were renderer strings), so it was ruled
on its own. What landed, in `crates/backtest/src/sweep_harness.rs`:

- **The falsified fee-bleed claim is WITHDRAWN, not softened.** The MN conclusion now states that
  the report does not identify the binding cost, carries the measurement that refutes the old
  reading (uniformly FRAGILE at 0 bps; 0 → 5 bps costs ~0.05–0.16 Sharpe against a ~0.3 gap to the
  band), and adds an explicit `Supersession (bug-log #110)` line. A 0-bps surface additionally says
  so of itself: *"This surface IS the 0 bps read: fee-bleed is excluded by construction here."*
- **The § 0 dollar-neutral null is rendered at last** as a `NULL-0` row, analytic and verdict-free,
  quoting ADR-0051 § D-MN. BUYHOLD is kept but explicitly demoted — deleting a measured control
  loses information; leaving it unlabelled let it be read as the bar.
- **`trades` and `funding` columns added**, so R-MN.3's net-of-cost read is derivable from the
  report instead of from outside it. The `trades` header says in the body that the count **includes
  synthetic liquidation covers**, which is the comparability trap made visible rather than removed.

**Two negative controls, both measured, not assumed:**

1. **Non-MN bodies are untouched.** Two non-MN surfaces (`ts-horizon-daily-2023`,
   `carry-horizon-4h-2023`) were re-run through the changed renderer: **both body-SHAs
   byte-identical.** Independently, with the 12 new MN bodies in place and `anchors.toml` still
   untouched, the gate reported exactly **12 FAIL / 107 PASS**.
2. **The renderer moved no number.** Every shared θ-cell column across all 12 surfaces is
   **byte-identical to the first pass** — the change is presentation, not computation.

`anchors.toml`: 119 rows before and after, none added, none deleted, **12 SHAs changed (all
`v2-mn-*`), 107 byte-identical, 0 namespaces changed**. **ANCHORS PASS (119 / 119)**; spec-lint
PASS; adr-registry PASS; `p2_verdict_rerun` 3/3, `null_data_no_crown` 8/8, backtest lib 298/298;
clippy clean on the touched crate.

**Still open, and deliberately so:** the **ruin count**. Making a wiped-out path distinguishable
from a merely-bad one needs a counter threaded through the path loop, not a renderer line, so it
is out of this pass's scope and is not claimed as delivered. Same for separating synthetic
liquidation covers from real fills in `trades` — the body now *says* they are conflated, which is
the honest interim, not the fix.

---

**Original entry, kept for the record:**

**Status**: OPEN — found 2026-09-25 while answering story 1-26's AC6 (unblock 1-21).
Anchor-impacting: **yes if fixed** — the strings live in the hashed body, so clearing them
re-locks the 12 MN surfaces a second time. Needs its own ruling; **not** a D6.b case (D6.b
covers bodies reflecting a *wiring bug*, and this is a renderer string).

Story 1-21's review listed riders "owed at regeneration". The regeneration has now run, and it
cleared the ones that were **arithmetic** and none of the ones that were **prose** — because
re-running a corrected engine through an unchanged renderer cannot change what the renderer says.
Nobody had separated the two classes, so all of them were booked against the re-lock.

**Cleared by the re-lock — measured on the clean corpus:**

| rider | before | after |
|---|---|---|
| C1 `#75` — basis arm never saw the basis | `mn-basis` ≡ `mn-funding`, **8 of 8** compared cells bit-identical | **0 of 8** identical |
| `#71` absorbing liquidation state | **2210** liquidation events across 24 cells | **0** |
| the 97.8–100 % `p95_maxdd` it was offered to explain | 85.51 – 100.00 % | **16.64 – 34.74 %** |
| `git_commit` stamp naming a commit without the harness | `18334c9` | `2d63ddef`, the commit that produced them |

**NOT cleared, and now frozen into a third generation:**

- **The falsified fee-bleed sentence.** All 12 bodies still close with *"The dollar-neutral
  construction removes directional beta but not fee-bleed from short-leg turnover."* The clean
  corpus falsifies it outright, using the surfaces' own numbers: **at 0 bps taker fee — no fee
  bleed at all — every MN surface is still FAMILY-UNIFORM-FRAGILE and every cell FRAGILE.** Best
  p50 at 0 bps is **+0.2038**, against a FRAGILE band of 0.5 and a buy-and-hold control near
  +1.10…+1.74. Going from 0 to 5 bps costs only **0.05–0.16** Sharpe. The fee is not what is
  killing this arm, and six of the twelve surfaces were run at zero fee precisely to check that.
- **The § 0 null was declared and still is not rendered.** § 0 replaced the BUYHOLD control with a
  dollar-neutral ≈0 null because benchmarking a beta-stripped arm against +1.74 is the wrong null.
  All 12 bodies still print **BUYHOLD** and **none** contains a ≈0 null row. (A grep for
  "dollar-neutral" hits — but only the prose sentence above, not a control row. Worth recording as
  its own small trap: the string that looks like the fix is the string that is the defect.)
- **Body hygiene**: still **no `trades` column** and **no funding-cost column** on any MN surface,
  so R-MN.3's "net-of-cost edge at each fee level" remains underivable from its own evidence; still
  **no ruin count**, so a wiped-out path stays indistinguishable from a merely-bad one.

**Consequence for 1-21.** Its two CRITICALs are resolved at the arithmetic layer and its FRAGILE
verdicts now stand on their own merits rather than on a liquidation artefact — but it still cannot
close, because three of the riders its review made conditions of closure are untouched, and one of
them is a false statement inside anchored evidence.

**The lesson worth keeping.** "Owed at regeneration" was the wrong bucket. A rider is only
discharged by a re-run if the re-run changes the thing the rider is about. Riders against the
*renderer* need a renderer change and their own re-emission; booking them against a compute window
made them look scheduled when nothing was going to touch them.

### `#111` — the `#67` blast radius was scoped by LANE, and `#67` was fixed BENEATH the lanes
**Status**: OPEN — cause bisected 2026-09-25 (story 1-27). Resolution is a D6.b re-lock, not a code
fix; the scope of that re-lock is the open question.
Anchor-impacting: **yes** — at minimum the 4 scenarios below; possibly more (see § Scope).

The four `#[ignore]`d `determinism.rs` gates have been red since August and were believed to be a
separate, older drift of unknown cause. They are not. Bisected, one commit either side, on the
default invocation those tests use:

| commit | date | produces |
|---|---|---|
| `b3332d35` | 2026-08-15 | `0f6f6eb8…` — **GOOD**, reproduces the pin exactly |
| `11acd126` | 2026-08-16 | `b655e5e7…` — **BAD**, and it is the hash we still see today |

`11acd126` is **the `#67` fix**: *"the harness was booking a ~1% gain for buying one symbol at
another symbol's price — engine guard + per-symbol fill routing"*.

**So the pinned bodies are `#67`-contaminated evidence, and the gates are red because the engine
became CORRECT.** Story 1-27 is therefore a D6.b re-lock, not a bug hunt. The diff says the same
thing in numbers — on `top10-2023-1h-momentum`, 8 changed lines, all summary-table:

| field | pinned (contaminated) | produced (clean) |
|---|---|---|
| Trades | 4809 | 592 |
| Buys / Sells | 2406 / 2403 | 296 / 296 |
| Max drawdown | 87.63 % | 14.34 % |
| Final equity | $50 922.49 | $87 606.01 |

**The reasoning error, which matters more than the four SHAs.** Every artefact that touched this —
`determinism.rs`'s own comment, story 1-26's Task note, bug-log `#109`, the 1-26 errata — excluded
`#67` on the same ground: *these four do not go through `run_path`*. That is **true and
irrelevant**. `#67`'s fix landed in `engine.rs` (+20) and `paper.rs` (+98) — the engine **below**
every lane. Anything that calls `engine.step` with a multi-symbol universe is in scope, `run_path`
or not.

A defect fixed beneath the lanes cannot be bounded by naming lanes. The `#67` inventory was built by
naming lanes (`run_path`, `run_cell` → 34 θ-surfaces), so it bounded the wrong thing.

Corroborating detail, not decoration: the `t622_*` gates (`btc-2023-1m-sma-cross` and siblings) stayed
**green** across `11acd126`. They are single-symbol, and "buying one symbol at another symbol's price"
needs at least two. The defect's own mechanism predicts exactly which anchors move, and it is not
"the ones on `run_path`" — it is "the multi-symbol ones".

#### Scope — the part that is not yet measured

77 distinct scenarios are anchored. 34 are the `#67` inventory; 4 are these gates. Of the remaining
**39**, thirteen are multi-symbol by name and therefore candidates on the mechanism above:

- 11 × `top10-*` (momentum-realdata, patchtst-overlay, regime-dispatcher ×2, tcn-overlay-realdata ×2,
  tcn-overlay-weights ×2 + ×2 realdata, vol-target-overlay)
- 2 × `pairs-*` (`pairs-2023-zscore-mr`, `pairs-2024-h1-zscore-mr`)

**Nothing in the repo can currently say whether they moved**, which is bug-log `#93`'s point exactly:
`verify_anchors.sh` hashes committed bodies and never re-runs, and only these four scenarios have a
re-run gate at all. The 26 single-symbol / non-engine scenarios are out of scope on the mechanism.

#### Scope — first measurement, 2026-09-25

Two of the thirteen were re-run at `HEAD` on the default invocation and compared against every
anchored row for their scenario:

| scenario | produced | verdict |
|---|---|---|
| `pairs-2023-zscore-mr` | `ac647a59…` | matches **neither** `v1.5a + noop-baseline` (`90591a0e…`) nor `v1.5a + v5-realdata-medium-2026-05` (`01c9da4d…`) — **DRIFTED** |
| `pairs-2024-h1-zscore-mr` | `5bee5e9c…` | matches **neither** (`14f50a59…` / `6252819b…`) — **DRIFTED** |

Both are 2-symbol scenarios — the minimum the `#67` defect needs. Attribution for these two is **not
bisected**, so `#67` is the leading explanation on mechanism, not a measured cause; what *is*
measured is that they no longer reproduce.

`top10-2023-fy-tcn-overlay-weights` and its 2024 sibling could not be run by the default binary
(RUN-FAIL — they need the `candle` feature), so they stay unmeasured rather than counted either way.

**The headline, and it is a gate problem before it is a corpus problem:** `scripts/verify_anchors.sh`
reports **ANCHORS PASS (119 / 119)** right now, while **at least 6 anchored scenarios do not
reproduce** — the 4 gated ones and these 2. The gate hashes committed bodies and never re-runs, so it
cannot see this. That is bug-log `#93` stated as a number instead of a worry.

The remaining 11 candidates are unmeasured. The honest statement is: **the `#67` re-lock covered 34
of an unknown total; 6 further anchored scenarios are confirmed non-reproducing and 11 more are
unexcluded.**

#### COMPLETE MEASUREMENT — 2026-09-26, the operator's "measure everything first" satisfied

Every anchored scenario the `backtest` binary can run has now been measured under a declared
condition. Superseding every partial tally above.

**15 distinct anchored scenarios are gate-confirmed NOT to reproduce:**

| group | n | note |
|---|---|---|
| `top10-{2023-1h,2024-h1}-momentum`, `top10-{2023,2024}-fy-tcn-overlay` | 4 | cause **bisected** to `11acd126` (`#67`) |
| `pairs-{2023,2024-h1}-zscore-mr` | 2 | gated 2026-09-26; `ac647a59…`, `5bee5e9c…` |
| `top10-{2023,2024}-fy-tcn-overlay-realdata` | 2 | `b6d88fa6…`, `b5efe7b6…` |
| `top10-{2023,2024}-fy-tcn-overlay-weights-realdata` | 2 | `fa09a769…`, `0d6cc994…` |
| `top10-{2023,2024}-fy-tcn-overlay-weights` (m3) | 2 | not `#[ignore]`d — red for anyone building `--features candle` |
| `top10-2023-fy-momentum-realdata` | 1 | `0fc591e5…` vs pin `0867d232…` |
| `top10-2023-fy-patchtst-overlay-realdata` | 1 | `f704c4f2…` vs pin `b015b564…` |
| `top10-2023-fy-vol-target-overlay-realdata` | 1 | `91848e23…` vs pin `6adc4334…` |

**7 are gate-confirmed to reproduce:** `btc-2023-1m-sma-{cross,baseline-refresh}`,
`top10-{2023,2024}-fy-regime-dispatcher-realdata` (**new, and the first green ones in the `-realdata`
family** — see `#120`), `report-sample-{7d,90d}`, `btc-yahoo-2024-1d-sma-cross`.

**4 have a re-run gate that pins something which is not an `anchors.toml` row** — the three
`btc-2023-1m-{macd-trend,rsi-reversion,bbands-mean-revert}` synthetic pins and
`eth-yahoo-2024-1d-sma-cross`. Those four anchor rows have no coverage.

**~~The rest — 34 θ-surfaces, 1 MC scenario, 15 forecast/report-binary scenarios — still have no
re-run gate of any kind.~~ UPDATED 2026-09-27: the 34 θ-surfaces now have one.**
`crates/backtest/tests/theta_surface_reproduction.rs` re-runs each surface from
`scripts/relock/surfaces.tsv` and compares against its anchor. **All 34 measured GREEN** — 32 cheap in
**764 s**, the 2 expensive in **3742 s** (split into their own gate so the cheap block stays usable).
The expensive pair's runtime lands within 23 seconds of the 1461 s + 2258 s measured during the 1-26
regeneration two days earlier, which is a small independent check that the sweep is as deterministic in
cost as it is in output.
That takes the corpus from 12 anchor-comparing scenarios to **46**, and the largest single block of
`#93`'s blind spot is now observed rather than assumed.

Built to this week's rules rather than from scratch: the condition is **declared** (release binary,
`--features candle,realdata`, CWD = workspace root, invocation from the `GridKind`-derived manifest
that `relock_manifest.rs` already gate-tests); `--out-dir` always points at a tempdir, because the
binary's default points *inside* the anchored corpus and `verify_anchors.sh` takes the newest match
(`#113`); an absent corpus is **UNMEASURED and loud**, never a pass (`#113` req 6); and a killed
process is distinguished from a refusal (`#124`).

And it has the companion the audit named as the pattern to copy —
`corpus_gated_theta_tests_are_declared`, corpus-independent so it **always** runs, asserting the
manifest row count, that **every** surface has an anchor, and that the cost split is not stale; it
warns *louder* when the corpora ARE present, because then the gates should be run. Both failure modes
were probed rather than assumed: falsifying the row count fires, and mistyping an expensive scenario
fires with *"the cost split is stale"*.

Still without any re-run gate: **1 MC scenario + 15 forecast/report-binary scenarios.**

So the `#67` re-lock covered **34 surfaces of a corpus in which 15 further scenarios do not
reproduce** — and `verify_anchors.sh` reports `ANCHORS PASS (119 / 119)` through all of it. (On what
that number does and does not mean, see `#115`'s sibling finding: it prints `total / total`.)

### `#112` — one anchored scenario name, two legitimate bodies, selected by CWD — and a re-run gate that does not pin it measures a coin flip
**Status**: OPEN as a design constraint — found 2026-09-26 by walking into it. Not a code defect:
the behaviour is intended and the body even labels itself. The defect is that **nothing enforces
which condition a reproduction check runs under**, and the operator-approved reproduction gate
(story 1-27 follow-on) is exactly the thing that must.
Anchor-impacting: no.

`btc-2023-1m-sma-cross`, same binary, same `--seed 0xC0FFEE`, same commit — **20 of 37 body lines
differ** depending only on the working directory:

| | CWD = tempdir | CWD = repo root |
|---|---|---|
| Data source | `synthetic (seeded RNG, v0 fallback)` | `real (Binance Vision)` |
| Bars replayed | 525 601 | 17 544 |
| Total return | −82.01 % | +7.38 % |
| Max drawdown | 82.07 % | 4.20 % |
| Trades | 12 077 | 441 |
| body-SHA | `d2fa7616…` | `9b35c926…` |

`d2fa7616…` is what the `t622_*` gate pins, and the tempdir run reproduces it **byte-exactly**. The
repo-root run is not drift — it is a different, equally real answer to a different question, because
from a tempdir the parquet lookup misses and the v0 synthetic fallback takes over. `determinism.rs`
documents this for `t622_*` ("Re-locked to SYNTHETIC SHA … CWD=tempdir → parquet lookup misses"), and
the hashed body carries its own `Data source` line, so a human reader is never misled.

**An automated check is.** A corpus-wide reproduction sweep run from the repo root reported
**6 false `DRIFTED`** verdicts tonight — `btc-2023-1m-sma-{cross,baseline-refresh}`,
`btc-2023-1m-{macd-trend,rsi-reversion,bbands-mean-revert}` (these three matched by luck, having no
parquet for their symbol) and `eth-2024-h1-sma-cross` — before the confound was caught by re-running
one of them from a tempdir. Which condition is correct is **per scenario family**: `t622_*` anchors
are synthetic, `-realdata` anchors are real.

Worth noting what the bad measurement nearly cost: it appeared to **falsify** the `#111` mechanism
("single-symbol scenarios are immune to `#67`") by showing single-symbol SMA scenarios drifting. The
prediction is fine; the measurement was not. A prediction refuted by an unpinned measurement is not
refuted.

**Binding requirement for the reproduction gate:**

1. The gate declares, per scenario, which data condition its anchor was locked under — and runs it
   under exactly that one.
2. The gate asserts on the body's own `Data source` line **before** comparing SHAs, so a
   silently-changed condition fails loudly as a condition mismatch instead of as a fake drift.
3. A scenario whose condition cannot be established is reported as **unmeasured**, never as green.

Without (1)–(3) the gate reproduces this entry's failure at CI scale, which is worse than the
`#93` blind spot it exists to close: a gate that cries drift is retired by the third false alarm.

#### A SECOND confound, found the same way — the condition is CWD × FEATURE SET

After fixing the CWD, the tempdir pass reported the *opposite* set of false verdicts:
`btc-2023-1m-{bbands-mean-revert,macd-trend,rsi-reversion}` and `eth-2024-h1-sma-cross` as DRIFTED,
while the two that had failed from repo-root now reproduced. Neatly inverted — which is itself the
tell.

Cause: my binary was built `--features candle,realdata`; `determinism.rs::run_scenario_once` builds
plain `cargo build --bin backtest`, **no features**. The file says so — *"These anchors capture the
`PassthroughForecaster` path (candle feature absent in CI)"*. So the reproduction condition is at
least **(CWD, feature set)**, not CWD alone.

#### The authoritative measurement — the repo's own gate

Stopping the re-implementation and running `cargo test -p backtest --test determinism` settles it:

```
test result: ok. 16 passed; 0 failed; 4 ignored
```

All twelve `t622_*` / `t717_*` gates (sma-cross, sma-baseline-refresh, macd-trend, rsi-reversion,
bbands-mean-revert) **pass**. The only red ones are the four `top10-*` gates, already `#[ignore]`d
with the `#67` cause bisected. **Every DRIFTED verdict my sweeps produced for the `t622_*`/`t717_*`
families was false, in both directions.**

#### The lesson that outranks both confounds

I re-implemented a measurement apparatus that already existed, correct, in the repo — twice — and
each re-implementation silently answered a different question. `determinism.rs::run_scenario_once`
already encodes the condition (tempdir, no features, fixed seed, copied `config/strategies`); it took
two false result sets to go back and just run it.

**So the reproduction gate is an EXTENSION of `determinism.rs`, not a new sweep script.** Add
scenarios to the existing harness, with a per-scenario condition where it differs (a `-realdata` arm
needs the corpus and the features, so it needs its own runner alongside `run_scenario_once`, not a
shell loop around the binary). Requirements (1)–(3) above stand; requirement (0) is: extend the
harness that is already right.

#### Honest standing bilanz (2026-09-26)

| class | count | state |
|---|---|---|
| reproduce, proven by the repo's own gate | 12 | green |
| do not reproduce, cause bisected to `#67` | 4 | the `top10-*` gated ones |
| do not reproduce, measured twice under the no-feature tempdir condition | 2 | `pairs-2023-zscore-mr`, `pairs-2024-h1-zscore-mr` — same hash from both passes, so features do not move them; attribution to `#67` is mechanism, not bisect |
| condition never established, therefore **unmeasured** | 9 | the `top10-*-realdata` family (+ 2 `-weights` that RUN-FAIL without `candle`) |

**6 confirmed non-reproducing, 12 confirmed reproducing, 9 genuinely unknown** — and the 9 are
unknown because no gate declares their condition, which is the same defect this entry is about.

#### Update 2026-09-26 — the gate was built, and it measured

Four R-REPRO gates were added to `determinism.rs` (compare against the anchor, `--reports-dir` to a
tempdir, precondition absence panics as UNMEASURED). Running them:

| scenario | expected (`v5-sqrt-impact-2026-05`) | produced | verdict |
|---|---|---|---|
| `top10-2023-fy-tcn-overlay-realdata` | `1157af76…` | `b6d88fa6…` | **DRIFTED** |
| `top10-2024-fy-tcn-overlay-realdata` | `39a02c79…` | `b5efe7b6…` | **DRIFTED** |
| `top10-2023-fy-tcn-overlay-weights-realdata` | — | — | UNMEASURED → unblocked by `#114` |
| `top10-2024-fy-tcn-overlay-weights-realdata` | — | — | UNMEASURED → unblocked by `#114` |

So the running count is **8 confirmed non-reproducing** (4 `top10-*` in-test, 2 `pairs-*`, 2
`tcn-overlay-realdata`), 12 confirmed reproducing, and the remainder split between the 2 weights arms
(unblocked by the `#114` fix) and **5 with no runner at all** —
`top10-2023-fy-momentum-realdata`, `top10-2023-fy-patchtst-overlay-realdata`,
`top10-{2023,2024}-fy-regime-dispatcher-realdata`, `top10-2023-fy-vol-target-overlay-realdata`.

A caveat kept deliberately: my earlier shell sweep reported `momentum-realdata` (`0fc591e5…`) and
`patchtst-overlay-realdata` (`f704c4f2…`) as drifted. **Those numbers are not counted as confirmed.**
They came from the same class of apparatus that produced two sets of false verdicts (`#112`), so they
are a hypothesis until a gate covers those scenarios.

#### Final tally for this pass — 2026-09-26, gate-measured only

With `#114` fixed the two weights arms ran, and the `m3_*` gates (which were never `#[ignore]`d and
fail for anyone running `--features candle`) were measured too, after their runner was given the same
`--reports-dir` fix:

| scenario | expected | produced |
|---|---|---|
| `top10-2023-fy-tcn-overlay-weights-realdata` | `38736839…` | `fa09a769…` |
| `top10-2024-fy-tcn-overlay-weights-realdata` | `582dabab…` | `0d6cc994…` |
| `top10-2023-fy-tcn-overlay-weights` (m3) | `7cb1357c…` | `175173b6…` |
| `top10-2024-fy-tcn-overlay-weights` (m3) | `23c24dae…` | `3c1178fb…` |

**12 distinct anchored scenarios are gate-confirmed NOT to reproduce:**

- 4 `top10-*` in-test pins — cause bisected to `11acd126` (`#67`)
- 2 `pairs-*` — measured twice under the no-feature tempdir condition
- 2 `top10-*-fy-tcn-overlay-realdata`
- 2 `top10-*-fy-tcn-overlay-weights-realdata`
- 2 `top10-*-fy-tcn-overlay-weights` (m3)

**~~5 are gate-confirmed to reproduce~~ — CORRECTED 2026-09-26: only 2 reproduce an `anchors.toml`
row.** The `btc-2023-1m-{macd-trend,rsi-reversion,bbands-mean-revert}` gates pin `4d8192af` /
`4a744788` / `5037accb`, and **none of those three strings appears in `anchors.toml`** (verified,
0 occurrences each). They pin the *synthetic* body while the canonical row is the *real-data* body
(17 544 bars vs 525 601) — deliberate and documented (`determinism.rs:538-541`, ADR-0045 § D6.3), but
the narrow consequence stands: **those three anchor rows have no re-run coverage.** Only
`btc-2023-1m-sma-{cross,baseline-refresh}` pin `d2fa7616`, which *is* an `anchors.toml` row.

The count of genuinely-green anchor coverage is nevertheless still 5, because the coverage audit found
three real gates nobody had counted: `report-sample-{7d,90d}` and `btc-yahoo-2024-1d-sma-cross` (the
latter in CI). See [`anchor-gate-coverage-audit-2026-09-26.md`](anchor-gate-coverage-audit-2026-09-26.md).

**5 anchored scenarios have a binary that can run them but no gate**, so they stay unmeasured:
`top10-2023-fy-momentum-realdata`, `top10-2023-fy-patchtst-overlay-realdata`,
`top10-{2023,2024}-fy-regime-dispatcher-realdata`, `top10-2023-fy-vol-target-overlay-realdata`.

And **`scripts/verify_anchors.sh` reports `ANCHORS PASS (119 / 119)` throughout.** Twelve scenarios do
not reproduce and the corpus gate is green, because it hashes committed bodies and never re-runs.
That is `#93`, now with a number on it.

**A demonstration, not a hypothetical:** the `m3_*` run above was RED, and before it ran its runner
was given `--reports-dir`. Without that fix the red run would have written two drifted bodies into
`evidence/<feature>/reports/`, where `verify_anchors.sh` takes the newest — flipping the corpus gate
to FAIL as a side effect of running a test. `git status -- evidence/` was empty after the run.

### `#113` — the `-realdata` anchors have a test named `_determinism` that delivers determinism, and a data-absent path that returns green having measured nothing
**Status**: OPEN — found 2026-09-26 while establishing the reproduction condition for the 9 unmeasured
`-realdata` anchors (bug-log `#111`). This is the concrete design basis for the reproduction gate.
Anchor-impacting: no by itself; **yes as a side effect** — see § The landmine.

`crates/backtest/tests/determinism.rs` carries four `#[cfg(feature = "realdata")]` tests —
`realdata_{2023,2024}_fy_tcn_overlay_determinism` and their `_weights` siblings. Each one:

```rust
let report1 = run_realdata_scenario_once(&bin, &workspace, scenario);
let report2 = run_realdata_scenario_once(&bin, &workspace, scenario);
…
assert_eq!(hex1, hex2, "…body-SHA256 must be identical across two runs…");
```

It runs the scenario **twice and compares the two runs with each other**. It never compares against
`anchors.toml`. So it passes for as long as the engine is deterministic — **including when the code
has stopped reproducing the anchored body.** The name is honest about what it does; the mistake is
reading it as coverage for reproduction. It is the `#93` blind spot with something reassuring parked
next to it.

Second half: when `data/binance/REVISION.toml` is absent the test prints a message and `return`s —
commented *"soft skip — does not count as failure"*. In CI, which does not carry the 240 parquets,
these four therefore report **green having executed nothing**. Both halves together mean the
`-realdata` family has never had a reproduction check, in CI or locally.

#### The landmine

`run_realdata_scenario_once` runs with `current_dir(workspace_root)`, so the report lands in
`evidence/<feature>/reports/` — **inside the anchored corpus.** `verify_anchors.sh` resolves each
anchor to the *newest* matching report in that directory. So if these tests are ever run at a commit
whose output differs from the pin, the run **plants a drifted body in the corpus and flips
`verify_anchors.sh` to FAIL as a side effect** — a test whose execution can break a different gate.

`--reports-dir` exists precisely for this and its own help text says so ("Useful for re-running into a
tempdir without touching the anchored reports under `evidence/`"). These tests predate it, or missed
it. `run_scenario_once` (the non-realdata sibling) is clean — it uses a tempdir.

#### What the reproduction gate must therefore do

Adding to `#112`'s requirements, and now from measurement rather than principle:

4. **Compare against the anchor, not against a second run of itself.** Self-consistency is a
   different property and both are worth having, but only one of them can see drift.
5. **Never write into `evidence/`** — pass `--reports-dir` to a tempdir. A gate must not be able to
   damage the corpus it checks.
6. **A skip is not a pass.** Data-absent must be reported as `unmeasured` and counted, not `return`ed
   silently green. (`#104` is the same shape: a secrets gate that swallowed the absence of its
   scanner.)

The good news is how little is missing: `ensure_realdata_binary`, `run_realdata_scenario_once`,
`real_binance_data_available` and `tcn_checkpoint_present` already encode the condition correctly.
**What is missing is the comparison.** That is a surgical change, and making it is also how the 9
unmeasured anchors of `#111` finally get measured.

### `#114` — a checkpoint guard that looked for a filename shape that has never existed, so two tests skipped silently and reported green for four months
**Status**: FIXED 2026-09-26 (resolver corrected); the measurement it was hiding is in `#111`.
Anchor-impacting: no.

`determinism.rs::tcn_checkpoint_present(name)` built:

```rust
.join(format!("{checkpoint_name}.safetensors"));
if !ckpt.exists() { eprintln!("… absent (LFS not resolved) — skipping weights test"); return false; }
```

The real convention is `<name>-<content-hash>.safetensors`. On disk:

```
crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.safetensors   1.67 MB, resolved
```

`tcn-bs1.safetensors` has never existed. So the guard returned **false on every machine**, LFS
resolved or not, and the two tests behind it —
`realdata_{2023,2024}_fy_tcn_overlay_weights_determinism` — `return`ed before their first assertion
**every time they were invoked, since `ce4ccbdd` (2026-05-18)**. Four months of a green test result
for code that never ran.

The diagnostic made it worse than silent: it printed *"(LFS not resolved)"*, naming a cause that was
not the cause. Anyone who checked would have found the checkpoint present, concluded the message was
stale, and moved on. **The guard accused the data of being absent when the bug was in the accuser.**
This entry's own first draft repeated that diagnosis, because the new R-REPRO gates inherited the
same helper — the false message propagated into a new test the same day it was written.

**Fix**: match `<name>-*.safetensors` by prefix, and require a file larger than 4 KiB so an
unresolved LFS pointer (~130 bytes) is still correctly reported as absent. The success path now
prints the resolved path, so a future reader can see *which* file satisfied the guard rather than
trusting that one did.

**The shape to recognise**, since this is the fifth instance this session (`#102` unreachable
feature, `#104` secrets gate with no scanner, `#113` data-absent soft skip, `#112` unpinned
condition, and this): **a precondition check that cannot succeed is indistinguishable from a
precondition that is never met.** Both look like a clean skip. The distinguishing move is to make
the *success* path observable — log what satisfied the guard, and count skips as `unmeasured`
rather than folding them into a pass.

### `#115` — the linter that guards the anchor pins is blind to 11 of 25 of them, and reports "0 skipped"
**Status**: FIXED 2026-09-27. And fixing the blindness immediately exposed that the rule underneath
it was wrong too — see § The second half.
Anchor-impacting: no directly; **yes indirectly**, since it is what is supposed to notice a stale pin.

`scripts/check_determinism_anchors.py` is ADR-0045 § D7.1's gate: every non-cfg-gated `const ANCHOR`
in `determinism.rs` must equal its `anchors.toml` SHA. Re-derived both numbers:

```
grep -cE "const (ANCHOR|EXPECTED)" crates/backtest/tests/determinism.rs   ->  25
python3 scripts/check_determinism_anchors.py
  check_determinism_anchors: OK — 14 literal(s) match (8 canonical, 6 synthetic; 0 skipped: cfg-gated)
```

A site only enters the script's `sites` list if `scenario_body_hex("…")` appears within 7 lines
(`:150`). Sites whose runner is spelled `scenario_body_hex_candle(` — the `m3_*` pair — or
`assert_reproduces_canonical_anchor(` / `assert_reproduces_or_report_unmeasured(` — **the nine R-REPRO
gates written today** — resolve to `scenario is None` and are dropped at `:196-204`. Not counted, not
warned, and **not reported as skipped**. 25 − 11 = 14, exactly the printed number.

**The 11 invisible sites are precisely the ones proven stale this week** (`#111`, `#112`): the two m3
weights pins and the nine `-realdata` reproduction pins. The gate whose job is to notice a pin going
stale cannot see the pins that went stale, and its own output says `0 skipped`.

Two smaller faults in the same file: a total regex failure prints `WARN` and `exit 0` (`:519-521`),
and `--write` (`:525`) syncs only the visible 14 — so an operator "fixing drift" with it silently
leaves 11 pins untouched. Unlike its five sibling gates it has **no `--self-test`**.

**Fixed 2026-09-27**, and the tool now reports
`OK — 25 of 25 resolved literal(s) match … 0 unresolved. Sites seen: 25 (floor 25).`

- **Resolve the SCENARIO, not the runner.** The regex matches any quoted argument with a scenario
  name's shape, so a new helper spelling needs no change here. The old regex made every new helper
  invisible **by default**, which is the property that turned a parser detail into four months of
  false confidence.
- **An unresolvable site is a hard failure**, listed with `file:line`, `fn`, and the literal. A total
  parse failure now returns 1 instead of `WARN … return 0`.
- **A non-vacuity floor** (`MIN_EXPECTED_SITES = 25`) with the count printed either way, so a silent
  shrink is visible and lowering it is a reviewable act.
- **`--self-test` with four probes**, because a linter without one is exactly what this entry is
  about: it reports a number nobody has checked it can fail to produce. Each probe corresponds to a
  way the tool was once silently wrong.

#### The second half — the blindness was hiding a wrong RULE

The moment the tool could see all 25 sites it reported **9 stale literals**. They are not stale. Its
mapping rule — *every in-test pin mirrors the `v5-realdata-medium-2026-05` row* — is namespace-naive:
7 of those sites target `noop-baseline` (the default real-data invocation is zero-sim-slippage,
`#123`) and 2 target `v3.0.0-regime`. **Running `--write` on that assumption would have re-pinned nine
CORRECT gates to the wrong rows — `#77` committed by the tool built to prevent it.**

So the fix is not only "see more sites". **The target namespace is a property of the site**, and the
site now says so: `// anchor-ns: <version substring>` next to its `const ANCHOR`, defaulting to the
canonical suffix so every pre-existing site keeps its meaning unchanged. Same principle as the gates
declaring their run conditions — the thing that knows is the thing that states it.

And `--write` **refuses** when any site declares a non-canonical namespace, naming them, rather than
syncing the ones it happens to understand.

#### One probe caught a bug in my own fix

The declaration search first scanned a wide window in both directions. Removing a declaration left the
tool **green** — the site had borrowed its neighbour's. A window wide enough to be forgiving is wide
enough to launder one site's meaning into another. It is now backwards-only, at most three lines, and
stops at a `fn` boundary.

That probe existed only because I ran it. The first attempt at it used `sed` expressions that silently
matched nothing, and both "probes" reported OK on an unmodified file — two no-ops I nearly recorded as
passes. **A probe you did not verify changed something is not a probe**, which is this entry's own
shape aimed at the person writing it.

### `#116` — two neutrality gates hash a committed file and compare it to a constant copied from that file, while the drift they guard is live
**Status**: OPEN — found 2026-09-26. Both gates are GREEN right now over a scenario measured as
drifted the same day.
Anchor-impacting: **yes** — both run with `current_dir(workspace_root)` and no `--reports-dir`, so
executing either can plant a body in the corpus (`#113`'s landmine, two more instances).

`crates/forecast/tests/patchtst_overlay_neutrality.rs:34,150,212` (K4, ADR-0036) and
`crates/trader/tests/llm_forecaster_neutrality.rs:36,141,224` (R10.2) assert that adding an overlay
must not change `top10-2023-fy-tcn-overlay-realdata`'s body-SHA.

They run the scenario — and then hash the wrong file. `report_dir_for_scenario` maps this scenario to
`evidence/backtest-real-binance-data/reports/` (`crates/backtest/src/main.rs:2569-2573` + `:2549-2552`)
and **that directory does not exist** (verified: `ls` fails; the corpus is at
`evidence/v1/backtest-real-binance-data/`). Both tests search only the `evidence/v1/…` candidates, so
`find_latest_report` resolves the newest **committed** body — whose SHA is bit-identical to
`EXPECTED_SHA`, because `EXPECTED_SHA` was copied from it. **The run's own output is never examined.**

The R-REPRO gate measured this scenario producing `b6d88fa6…` on the same day (`#111`). So the drift
is live, and two gates written to catch exactly this report green.

Compounding: `EXPECTED_SHA` is the `v2.6.0-realdata + noop-baseline` row — three generations behind
the live `v5-sqrt-impact-2026-05` row `1157af76…`.

**Fix**: do not repair the bespoke harnesses — delete them and add the two scenarios to
`determinism.rs::assert_reproduces_canonical_anchor`, which already writes to a tempdir, compares
against the current-generation anchor, and reports an absent precondition as unmeasured. This is
`#112` requirement (0): extend the apparatus that is already right instead of maintaining a second one.

### `#117` — `verify_anchors.sh` re-implemented in Rust, inside `cargo test --workspace`, named as if it re-ran anything
**Status**: OPEN — found 2026-09-26. A naming and framing defect, not a logic bug; the highest-value
one to act on because of who reads it.
Anchor-impacting: no.

`crates/reports/tests/strategy_anchors_unchanged.rs` holds three tests — `:436`, `:489`, `:542` —
checking 25 anchor rows across three namespace tables. All three read committed report bodies off
disk (`std::fs::read_to_string` at `:403`; verified — the file contains no `Command::new`), and the
module doc at `:29-31` says so itself: it *"mirrors `scripts/verify_anchors.sh:63-110`"*.

So it is the corpus gate re-implemented in Rust, **inside the workspace test suite**. That is worse
than `verify_anchors.sh` for one reason only: a *test* named `*_strategy_anchors_unchanged` reads as
a reproduction check. A developer running `cargo test --workspace` sees three green tests whose names
say the strategy anchors are unchanged — over exactly the scenarios that do not reproduce.

Two of the three additionally soft-skip on an empty table (`:490`, `:543`) — `#113` requirement 6 —
though all three tables are in fact populated today.

**Fix applied 2026-09-26 — and the obvious fix was the wrong one.** The audit recommended renaming to
`*_committed_bodies_unchanged`. **Rejected after checking:** the names are cited by name in
**byte-immutable** anchored reports (`evidence/v1/v2-llm-strategy/reports/test-2026-05-12-…`,
`evidence/v1/cockpit-toast-queue/reports/test-final-2026-05-27-…`) and in ADR-0043 § D-t1937. A rename
would orphan citations in evidence that cannot be edited to follow it (AD-2 / ADR-0038 § D6).

What landed instead:

1. **A `## What this test does NOT prove` block at the top of the module doc**, before the existing
   description — the shape `reproducibility_sample_figure.rs` uses and the audit itself praised. It
   states that these tests prove storage integrity and nothing about reproduction, gives the number
   (12 anchored scenarios do not reproduce while these are green), names where the gate that CAN see
   drift lives, and records that the rename was considered and why it was rejected. The name still
   reads wrong; now the first thing under it says so.
2. **Non-vacuity on both soft-skips** — `assert!(!TABLE.is_empty(), …)` before the `return`. Both
   tables were populated at v0.3.0 and v0.5.0, so emptiness today means the constant was deleted or
   renamed, which must be loud. `#113` requirement 6.

The general point is worth keeping: when a misleading name is load-bearing in immutable evidence, the
fix is to put the correction where the reader lands, not to break the citation chain.

### `#118` — an anchor outlived its producer, and the corpus gate has reported PASS on it every day since
**Status**: OPEN — needs a disposition decision, not a fix. Found 2026-09-26.
Anchor-impacting: **yes** — 2 rows.

`sharpe-comparison-realdata` is pinned twice in `evidence/anchors.toml`. **No code path at HEAD emits
the name** — verified: `grep -rl "sharpe-comparison-realdata" crates/` returns nothing.
`crates/forecast/src/bin/sharpe_comparison.rs` emits four names and the bare one is not among them.
The resolved report dates to 2026-05-19, and
`docs/dev-notes/retired-surface-inventory-2026-05-22.md:157` lists the surface as retired.

So the anchor outlived its producer, and `verify_anchors.sh` has reported PASS on both rows ever
since — because it hashes a file, and the file is still there. This is `#93` in its purest form: not
a gate that failed to notice drift, but a gate verifying evidence for an experiment that can no
longer be run at all.

The disposition is a decision, not a measurement. Either the rows are relabelled as historical record
carrying no reproduction claim, or the rename is traced and they are re-keyed to the surviving name.
**Re-emitting is not available.** Related, same shape and also unreachable:
`eth-yahoo-2024-1d-sma-cross`, whose anchored body contains a `rev=` substring that `D-V0.1.3-1`
moved out of the body — see the coverage audit § 3.

### `#119` — `#114`'s twin: the same wrong CWD assumption, the same wrong blame, in the forecast crate
**Status**: FIXED in both prescribed sites — `tcn.rs` resolves through `resolve_anchors_dir`
(CWD-relative first, `CARGO_MANIFEST_DIR` fallback, logs what it settled on) and `patchtst.rs` calls
`resolve_anchors_dir_pub`. Status corrected 2026-10-01; it had read OPEN after the fix landed.
**But the fix list below was ENUMERATED, not derived** — see the 2026-10-01 note at the end of this
entry and `#135`.
Anchor-impacting: no.

`crates/forecast/src/tcn.rs:496-497`:

```rust
// In tests and binaries, the CWD is the workspace root.
let anchors_dir = PathBuf::from("crates/forecast/checkpoints/anchors");
```

Cargo runs integration tests with CWD = the **package** root. This repo documents that in four
places, including `docs/dev-notes/bug-log.md:192` ("cargo runs ui test binaries with
cwd=`crates/ui/`" — empirically confirmed there). So under `cargo test -p forecast --features candle`
the path resolves to `crates/forecast/crates/forecast/checkpoints/anchors`, `load_anchor` returns
`CheckpointNotFound`, and `crates/forecast/tests/anchors_load.rs` skips on **every** machine while
printing *"run `git lfs pull` to fetch checkpoints"* — with the 1.67 MB checkpoint resolved on disk.

Character-for-character `#114`: a precondition that cannot succeed, plus a diagnostic that accuses
the data while the bug sits in the accuser. Found by looking for `#114`'s shape rather than by
tripping over it, which is the argument for writing these entries as shapes and not as incidents.

**Fix**: resolve from `env!("CARGO_MANIFEST_DIR")` in both `tcn.rs` and `patchtst.rs`, and print the
resolved path on success — `#114`'s own fix, applied to its twin.

**2026-10-01 — the fix list was two files because two files had symptoms.** `#126`'s lesson is that a
member list must be derived from the criterion, not written down; this entry's was written down. The
criterion is *"every workspace-relative path literal in a crate whose tests run from the package
root"*, and running it over the tree found **three more sites with this exact literal**, all in the
`*_readonly.rs` guards, all silently resolving to nothing — filed as `#135` and fixed the same day.
The rest of the sweep is clean and worth recording so nobody runs it again: `tcn_byte_identity.rs:113`
and `patchtst_byte_identity.rs:95` use the literal as a **git pathspec** against an explicit
`current_dir(&ws_root)` and keep a `checked_count` non-vacuity counter — the correct pattern;
`sigma_train_not_in_safetensors{,_patchtst}.rs` carry `"checkpoints/anchors"` as a second candidate,
which resolves from the package root; `vol_verdict.rs:131` is a clap default for a binary whose
declared CWD is the workspace root. Four sites, four different reasons they are fine, and none of
them is "it looked right".

---

## The shape, named — for the six entries above and the five before them

Eleven entries this week share one mechanism, and it is worth stating once rather than eleven times:

> **A gate that cannot fail is indistinguishable from a gate that passes.**

Two sub-shapes cover all of them. **(a) The expectation and the measurement come from the same
place** — `#116` hashes a committed file against a constant copied from it; the
`compute_robustness_flag` bit-identity test compares a function to its own delegate through a
hand-copied mapping. **(b) The gate reports a count it did not measure** — `verify_anchors.sh` prints
`total / total`; `check_no_clocks_in_ui_tests.sh` prints an array length, not files scanned;
`orch_determinism_check.sh` prints PASS having hashed zero files because the glob's error message is
itself deterministic.

The distinguishing move, in both cases: **make the success path observable.** Log what satisfied the
guard. Print how many files the walk read. Assert on *that* number — not on the absence of findings
in a set nobody proved was non-empty. The repo's own `crates/llm/tests/no_secrets_in_artifacts_test.rs`
does all three and documents why; it is the shape to copy, and it exists because `#104` forced it.

Full audits: [`docs/dev-notes/cannot-fail-gate-audit-2026-09-26.md`](cannot-fail-gate-audit-2026-09-26.md)
(14 CONFIRMED, 3 SUSPECTED, 10 counter-examples) and
[`docs/dev-notes/anchor-gate-coverage-audit-2026-09-26.md`](anchor-gate-coverage-audit-2026-09-26.md)
(all 77 anchored scenarios: 15 with real coverage, 4 off-anchor pins, 2 self-comparing, 56 with
nothing).

### `#120` — "the canonical namespace" is per SCENARIO, not one global choice, and pinning it globally reports a reproducing scenario as drifted
**Status**: FIXED 2026-09-26, caught by measurement before it shipped.
Anchor-impacting: no.

Writing the nine R-REPRO gates I pinned every `-realdata` scenario to its
`v5-sqrt-impact-2026-05` row, on the reasoning that this is the newest namespace and therefore "the
canonical one". Four of the five newly-gated scenarios have such a row, so the assumption held long
enough to look right.

Then the measurement came back:

| scenario | produced | which row that is |
|---|---|---|
| `top10-2023-fy-regime-dispatcher-realdata` | `f37bbb8d…` | **`v3.0.0-regime`** |
| `top10-2024-fy-regime-dispatcher-realdata` | `691a7056…` | **`v3.0.0-regime`** |

Both reproduce **exactly** — and both would have been reported as DRIFTED by my gate, which pinned
`857f9494…` / `519886dc…` (the sqrt-impact rows). Two perfectly reproducing scenarios, called broken
by the gate built to find broken ones.

What the default invocation reproduces is the `v3.0.0-regime` row. The sqrt-impact rows exist for
these two scenarios but **their invocation is not established** — presumably explicit `--sim-*` flags,
which is bug-log `#112`'s condition problem in yet another dimension.

**Fix**: both gates re-pinned from measurement to the `v3.0.0-regime` rows, `#[ignore]` removed — they
are real regression gates now, and the first two green ones in the `-realdata` family. The doc comment
records why the namespace is not the sqrt-impact one, so the next reader does not "correct" it back.

**The lesson, which is the same lesson again:** a scenario's canonical namespace is a property of the
scenario, and the only way to know which row the code reproduces is to run it and see. I picked the
newest-looking namespace uniformly because that is what "canonical" sounded like. Had these two gates
shipped `#[ignore]`d with a "known-red" label, the error would have been invisible — a wrong pin
hiding behind an expected failure.

### `#121` — a reproduction gate is only as runnable as its build profile, and the debug binary makes two of them unusable
**Status**: FIXED 2026-09-26.
Anchor-impacting: no.

`top10-2023-fy-regime-dispatcher-realdata` takes **270 s in release and over an hour in debug** —
measured: a debug run was killed after 1 h 25 m without finishing. The existing `*_determinism` tests
build the **debug** binary, which is fine for them because they are cheap. Un-`#[ignore]`ing the two
green regime-dispatcher gates on that binary would have made
`cargo test -p backtest --features realdata` unusable, which is how a gate gets disabled for good.

**Fix**: a separate `ensure_realdata_release_binary()` used only by the R-REPRO gates. The existing
`*_determinism` tests keep the debug binary.

Changing the profile is only safe because it was **measured, not assumed**, not to change the hashed
body: `top10-{2023,2024}-fy-tcn-overlay-realdata` and their `-weights` siblings each produced
byte-identical SHAs from a debug run and a release run — 4 scenarios, 8 runs. Independently
corroborated by the anchored report's own front-matter: it records `wall_clock_s: 3.2`, the release
run reports 3.2 s, and debug takes ~19 s. **The anchor was locked in release**, which was an inference
until the wall-clock confirmed it.

Verified on 4 of 9. If a future gate disagrees between profiles, the profile **is** part of its
condition and belongs in its doc comment — `#112`'s dimension list growing to
**CWD × feature set × build profile**, which is the fourth time this week that "the condition" turned
out to be wider than assumed.

**Cost, stated so nobody is surprised into `#[ignore]`ing them later:** the two green
regime-dispatcher gates take **602 s together** and are deliberately **not** `#[ignore]`d. That is
tolerable because `--features realdata` is not enabled in CI (see the coverage audit § 4), so they run
only when a developer explicitly asks for the realdata suite — and they are the only reproduction
coverage that family has. The alternative, if ten minutes ever becomes intolerable, is **not** to
silence them: it is the `dvol_bakeoff_path_gate.rs::corpus_gated_tests_are_declared_and_counted`
pattern — `#[ignore]` them, and add an always-running test that asserts the declared inventory matches
the `#[ignore]` count and prints the re-run command, so the skip stays visible. Its own docstring puts
it best: *"It fails when the skips become invisible."*

**And the `#119` fix recovered three gates that had never run.** With the checkpoint resolver
corrected, `cargo test -p forecast --features candle --test anchors_load` executes for the first time
since it was written: **3 passed, 0 failed** — checkpoint decode, `model_revision` prefix,
`sigma_train > 0` and forward shape, for both BS-1 and BS-2. They had been printing *"run `git lfs
pull`"* and returning green over a resolved 1.67 MB checkpoint. Fixing the accuser turned three
phantom passes into three real ones.

### `#122` — the `m3_*` gates build with `candle` and pin the PRE-candle re-emit, so their "drift" was measured against the wrong expectation
**Status**: OPEN — found 2026-09-26 while deriving the row mapping for the 1-27 D6.b re-lock.
Anchor-impacting: no by itself; it changes **which row** the re-lock moves for two scenarios.

`m3_top10_{2023,2024}_fy_tcn_overlay_weights_anchor_hash_unchanged` build the binary
`--features candle` (`run_scenario_once_candle`) and pin `7cb1357c…` / `23c24dae…`, which are the
**`v2.5.0-tcn-weights + noop-baseline`** rows.

The bodies do not state which namespace they belong to — every generation carries the same
`Slippage: 2 bps, Taker fee: 4 bps` line, so the friction text cannot distinguish them. **The
directories can**, and they settle it:

| sha | resolver directory | generation |
|---|---|---|
| `7cb1357c…` | `v1/v25-tcn-overlay/reports/` (5 copies) and `v5-latency-slippage-sim-v0.2.0-anchor-migration/` | the ORIGINAL, pre-candle |
| `28379df8…` | `v5-latency-slippage-sim-v0.4.0-**candle-feature-gated-re-emit**/reports/` | the candle re-emit |

A test that builds with `candle` should be pinned to the candle re-emit. It is pinned to the
generation before it. Same shape as `#120` — the wrong row of a multi-row scenario — and older.

**Consequence for the measurement:** the m3 pair is still DRIFTED (produced `175173b6…` /
`3c1178fb…`, which match neither candidate), so `#111`'s count of 15 is unchanged. But the *reported
delta* was against the wrong baseline, and the re-lock must move the **`v5-realdata-medium-2026-05`**
rows (`28379df8…` / `0c13ed0b…`), not the noop ones.

**Why it survived:** `check_determinism_anchors.py` skips these two as cfg-gated (`#115`) — so the one
gate whose job is to catch a pin pointing at the wrong row is precisely blind to them. Two findings
that only become visible together.

### `#123` — I read a comment for the behaviour, on the crux of a 35-row decision, and an independent derivation caught it
**Status**: pins CORRECTED 2026-09-26. One scenario remains BLOCKED on an operator ruling.
Anchor-impacting: **yes** — it determines which row the 1-27 D6.b re-lock moves for 7 scenarios.

Deriving which `anchors.toml` namespace row each re-emitted body replaces, I assigned the seven
`-realdata` scenarios to their **`v5-sqrt-impact-2026-05`** rows, citing `crates/backtest/src/main.rs:1641-1643`:

> Real-data: SquareRoot model + universe-avg V map. Synthetic: Linear{bps:8} fallback.

An independent derivation (deliberately not shown my table) read the **code that computes the model**
instead, and disagreed on 7 of 15. It is right. `build_slippage_model_for_scenario`
(`crates/backtest/src/main.rs:195-221`) returns, for a real-data scenario, `build_slippage_model(args)`
— **the CLI flags** — and both `sim_slippage_sqrt_alpha` and `sim_slippage_bps` default to **0**
(`:120-128`). So the DEFAULT real-data invocation produces `Linear { bps: 0 }`: **zero sim slippage.**

That comment describes the **v0.5.0 re-emission invocation**, the one run with
`--sim-slippage-sqrt-alpha 1.0`. Not the default. I read the intent and took it for the behaviour —
the exact failure mode of `#112`/`#113`/`#114`, committed by me, at the one point where 35 rows of a
byte-immutable corpus turned on it.

The syllogism that settles it, both halves verifiable:

1. **`noop-baseline` = the zero-sim-slippage generation** — stated in frozen evidence,
   `evidence/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/sharpe-delta-2026-05-29.md:17`:
   *"Noop baseline = pre-v5 zero-sim-slippage report"*.
2. **The default real-data invocation is zero-sim-slippage** — the code above.

So the produced bodies belong on the **`+ noop-baseline`** rows. Six gates re-pinned accordingly
(`8fa47f49`, `fd8191df`, `552d7df2`, `2a65c434`, `5f303cc0`, `9fa64d46`), and the assertion prose —
which named the sqrt namespace in a message that would have been printed to whoever hit it — corrected.

**What my rule needed and theirs did not: an exception.** I had to treat `#120`'s regime-dispatcher
pair as an exception, because it reproduced the *older* `v3.0.0-regime` row. Under the slippage rule it
is not an exception at all: a zero-slippage default run reproducing those rows **proves they are
zero-slippage bodies**. A rule that needs an exception to fit the one case you measured is usually the
wrong rule, and that was the signal I had and did not use.

**A measurement that closed one door and opened another.** Re-running
`top10-2023-fy-tcn-overlay-realdata` WITH `--sim-slippage-sqrt-alpha 1.0
--sim-slippage-sqrt-lookback-days 90` produces `48f1f25b…` — matching **neither** the sqrt row
(`1157af76…`) nor the noop row (`8fa47f49…`) nor the default run (`b6d88fa6…`). So the flags do move
the body (the condition is real), and **the sqrt row is drifted too**. For this whole family **no
constructible invocation reproduces any anchored row**, which means the row assignment cannot be
settled by reproduction — only by provenance. The independent derivation's `PROBABLE` is therefore the
correct confidence level and **cannot be raised by measurement today**; its own suggested upgrade test
presupposes an undrifted row, which is the thing that fails.

#### BLOCKED on a ruling — `top10-2023-fy-momentum-realdata`

Its **only** anchor row is `v5-sqrt-impact-2026-05` (`0867d232…`). The default invocation produces a
zero-sim-slippage body (`0fc591e5…`), and **there is no row for that condition.** Landing `0fc591e5`
on the sqrt row would make that row's own comment — which names
`SlippageModel::SquareRoot { alpha=1.0 }` — false.

Three ways out, none of them mine to pick:

- **(a)** Re-run it with the sqrt flags and land *that* body on the sqrt row. Keeps one row, one
  condition, and `0fc591e5` is then simply not the body to land. Needs the sqrt-flag run to be the
  one the gate asserts, i.e. the gate declares its flags.
- **(b)** Add a `+ noop-baseline` row for it. Clean semantically, but D6.b step 5 as executed for 1-26
  asserted **"none added"** — so this is a deliberate amendment to the invariant, not a detail.
- **(c)** Relabel the row as historical with no reproduction claim, per `#118`, and delete the gate.

> **RULED 2026-09-26 — (a).** Re-run it with the sqrt flags and land that body. Executed: the gate now
> **declares** its flags rather than inheriting the default, and measures
> `1fc0e85d14297bc48d683571811643a394e234187f09b95e41584be0bde337ef` against the pin `0867d232…`. So it
> is genuinely drifted under the *right* condition, and `1fc0e85d…` — not `0fc591e5…` — is the body the
> re-lock lands on the sqrt row.

The mechanism is now general and worth keeping: `assert_reproduces_with_flags(scenario, anchor,
declared_flags)`. **Passing `&[]` is itself a declaration** — "this gate asserts the default,
zero-sim-slippage condition" — so every gate states the invocation that produced the row it checks,
instead of one global assumption covering conditions that differ. That is `#112` requirement (1)
implemented rather than written down.

#### Final mapping — all 15, and the re-lock is smaller than it looked

One row moves per scenario, not all of them: **15 of 119 rows change, 104 stay byte-identical, 0
namespaces change.** The other 20 rows belonging to these 15 scenarios are frozen history and are part
of the negative invariant.

| target row | scenarios | how established |
|---|---|---|
| `+ v5-realdata-medium-2026-05` | 8 — momentum ×2, tcn-overlay ×2, pairs ×2, tcn-overlay-weights ×2 | synthetic ⇒ `Linear{bps:8}` forced regardless of CLI (`main.rs:214-220`); for the weights pair also directory provenance (`#122`) |
| `+ noop-baseline` | 6 — tcn-overlay-realdata ×2, weights-realdata ×2, patchtst, vol-target | default real-data ⇒ `Linear{bps:0}`, and noop **is** the zero-sim-slippage generation |
| `v5-sqrt-impact-2026-05` | 1 — momentum-realdata | its only row; gate declares the sqrt flags per ruling (a) |

**A landing constraint that must be honoured or the re-lock is invisible.** `verify_anchors.sh`
resolves each namespace to a different directory set, so a re-emitted body is not free to live
anywhere: canonical rows resolve from the `v0.4.0-candle-feature-gated-re-emit` / v0.3.0 migration dirs
(the global-newest fallback is never reached while those hit), the sqrt row resolves **only** from the
v0.5.0 dir, and noop rows resolve to the newest match *outside* all v5 dirs — by a lexicographic sort
of full paths, so the directory name outranks the timestamp. A body dropped in the wrong directory
changes nothing and the gate stays green over the old one.

### `#124` — a gate that was killed reported "the binary exited non-zero", which is a refusal the binary never made
**Status**: FIXED 2026-09-27, found by it happening during the 1-27 re-lock verification.
Anchor-impacting: no.

In the 754-second run of all nine R-REPRO gates, one failed:
`realdata_2023_fy_tcn_overlay_weights_reproduces_anchor`. Its message:

```
backtest binary exited non-zero for scenario top10-2023-fy-tcn-overlay-weights-realdata:
stdout:
stderr:
```

**Both streams empty.** That is the signature of a process killed by a signal — memory pressure, after
many hours of sustained load — not of a binary refusing a run. But the message says "exited non-zero",
which reads as a refusal, and the obvious next inference is drift in the very scenario the re-lock had
just re-priced. Re-run alone: **2 passed**. It was a kill.

This is `#114`/`#119` in the other direction. There, a guard accused the **data** while the bug sat in
the accuser. Here, a diagnosis accused the **binary** for something the environment did. Both are the
same defect: **a diagnosis that names the wrong cause is worse than none**, because it aims the next
reader away from the answer — and at `#114` that cost four months.

**Fixed**: the runner now prints the exit status explicitly and, when both streams are empty, says so:

> *"the process said NOTHING on either stream, which means it was almost certainly killed (signal /
> OOM) rather than refusing the run. Re-run before treating this as drift."*

#### The second finding, which is mine and older by two hours

The failure came from `run_realdata_scenario_once` — the **panicking** runner — because R-REPRO-1..4
used `assert_reproduces_canonical_anchor` (debug binary, panics on non-zero) while R-REPRO-5..9 used
`assert_reproduces_with_flags` (release binary, fallible, reports UNMEASURED). **One family, two
unstated conditions** — `#112`'s exact shape, written by me about two hours after I documented it.

All nine now go through one path: one runner, one declared condition, one UNMEASURED route, the release
binary. The old helper is deleted and a comment stands where it was, because the reason it existed is
the finding. Side benefit measured: the weights pair runs in **77 s** on release against 168 s on debug.

### `#125` — three scenarios had BOTH their anchor rows uncovered while a green `*_anchor_hash_unchanged` test sat beside each, and a fourth drift turned up whose cause is NOT `#67`
**Status**: 3 rows now covered and GREEN; 1 drift found and OPEN (unbisected). 2026-09-27.
Anchor-impacting: **yes for the fourth** — `eth-2024-h1-sma-cross` needs a re-lock once its cause is
known, and not before.

The anchor-gate coverage audit's "off-anchor pin" class, acted on.
`btc-2023-1m-{macd-trend,rsi-reversion,bbands-mean-revert}` each have two `anchors.toml` rows —
`noop-baseline` (synthetic, pre-friction) and `v5-realdata-medium-2026-05` (**real Binance Vision**) —
and their `t622_*`/`t717_*` gates pin a **third** value that is in neither
(`4d8192af`, `4a744788`, `5037accb`, held in `check_determinism_anchors.py::SYNTHETIC_DETERMINISM_SHAS`).
That is deliberate and documented (ADR-0045 § D6.3): those gates run from a tempdir, so the parquet
lookup misses and they assert the synthetic body.

The consequence nobody had acted on: **both real rows were uncovered**, while a green test named
`*_anchor_hash_unchanged` sat beside each one. The name is the trap, not the design.

**Measured under the minimal condition** — plain `backtest` binary, **no features**, CWD = workspace
root, `--reports-dir` to a tempdir:

| scenario | verdict | |
|---|---|---|
| `btc-2023-1m-macd-trend` | REPRODUCES | `6cb14ac5…` = its `v5-realdata-medium` row |
| `btc-2023-1m-rsi-reversion` | REPRODUCES | `87b4e1cc…` |
| `btc-2023-1m-bbands-mean-revert` | REPRODUCES | `5b6237d1…` |
| `eth-2024-h1-sma-cross` | **DRIFTED** | `405c2816…` against pin `bd4001e4…` |

Four `R-REAL` gates added. The three green ones assert the body's own
`Data source | real (Binance Vision)` line **before** the SHA (`#112` req 2), so a silent fall back to
the synthetic path reads as a condition mismatch rather than as drift.

#### The fourth one is the finding, and `#67` does not explain it

`eth-2024-h1-sma-cross` had **no** re-run gate at all — one real-data row, nothing re-ran it. It
drifts. And **`#67`'s mechanism predicts it should not**: `#67` was *"buying one symbol at another
symbol's price"*, which needs a multi-symbol universe. That is exactly why the single-symbol `t622_*`
family stayed green through `11acd126`, and that prediction is what made `#111`'s scope argument
credible in the first place.

`eth-2024-h1-sma-cross` is single-symbol, so the prediction said it should be immune.

> **DIAGNOSED the same day, and the prediction held.** The cause is the v5-latency-slippage-sim
> v0.5.0 **Q-D1=(a)** decision (`7e8a7e03`, 2026-05-29), which forces `Linear{bps:8}` on every
> scenario **not** listed in `REAL_DATA_SCENARIO_IDS` (`main.rs:43-59`, nine `-realdata` ids). This
> scenario is not listed, and its anchored body was generated **2026-05-28** — one day before the
> decision. The canonical-friction migration never swept it.

**No bisect was needed, because the diff plus arithmetic settled it** — the `#123` lesson applied.
The diff is 5 fields, and **trades / buys / sells are UNCHANGED**: same decisions, different costs,
i.e. a pricing change. Then the numbers close it:

| | |
|---|---|
| notional implied by the fees ($1697.19 at 4 bps taker) | **$4,242,981** |
| 8 bps on that — what Q-D1=(a) would add | **$3,394** |
| measured equity delta ($109,544.54 → $106,069.26) | **$3,475** |
| | **2.3 % apart** |

**Scope established as exactly one.** A scan for anchors whose resolving body predates 2026-05-29 and
sits outside the v5 migration directories returned 16 rows. Thirteen are `noop-baseline` — pre-friction
**by design**, and `anchors.toml:8-10` says so. Two run through `run_yahoo_sma`, a different binary
that Q-D1=(a) never touches — **proven, not assumed**, by `btc-yahoo-2024-1d-sma-cross`'s green CI
gate. That leaves `eth-2024-h1-sma-cross` alone.

Re-emitted under D6.b 2026-09-27: 119 rows before and after, **1** sha changed, 0 namespaces, 118
byte-identical, and exactly one row red before any sha moved. Its gate is now live and green.

Running count of non-reproducing anchored scenarios: 15 → 16 → **15 again**. And `determinism.rs`
now reports **24 passed, 0 failed, 0 ignored** — it began this week at 16 passed / 4 ignored.

#### Two linter corrections this forced

1. **An explicit `// anchor-ns:` declaration now beats the scenario-keyed synthetic override.** Two
   sites can pin the same scenario under different conditions — which is exactly the new arrangement
   for `btc-2023-1m-*` — and a scenario-level override compared the second against the first's
   expectation, calling a correct gate stale.
2. **The summary's "synthetic" count now follows the RESOLUTION, not the scenario name.** It was
   reporting 9 where the resolution used 6: a number nobody measured, in the summary line of the tool
   written to catch exactly that.


### `#126` — the `#67` inventory NAMED `run_path` and still missed a `run_path` scenario
**Status**: re-emitted and gated 2026-09-27. The last `backtest`-family scenario without a re-run gate.
Anchor-impacting: **yes** — 1 row, re-locked under D6.b.

`v1-momentum-2023-block-bootstrap-real-fy-mc` had exactly **one** generation (2026-05-30) and was not
touched by the 1-26 re-lock. Its state was therefore genuinely unknown, and the anchor-gate coverage
audit flagged it as the one non-θ scenario of the sweep family with no gate.

Measured 2026-09-27: **DRIFTED**, `7dbf5628…` → `3aae06c0…`.

**The attribution is mechanistic, not a resemblance.** `crates/backtest/src/mc_harness.rs:281` calls
`crate::scenarios::montecarlo::run_path` — the exact lane where `#94`'s sizer was wired (ADR-0089 D1,
`723ca742`) and where `#67`'s engine guard applies. The signature agrees independently: p50
`max_drawdown` **81.39 % → 34.12 %**, the same near-ruin collapse the 156 θ-cells showed
(85.53 % → 26.70 %).

That distinction matters, and it is the difference between this entry and `#125`. There, the mechanism
**predicted immunity** (single-symbol), so the resemblance contradicted it and folding it in would have
destroyed the evidence. Here the mechanism predicts involvement and the signature agrees. Same
observation, opposite handling, because the mechanism — not the resemblance — decides.

#### The finding is sharper than `#111`'s

`#111` said the `#67` inventory was scoped by **lane** while `#67` was fixed **beneath** the lanes — a
reasoning error. This is different and, in a way, worse: **the inventory named `run_path` explicitly,
and still omitted a `run_path` scenario.** The rule was right; the enumeration under it was
incomplete. A wrong rule announces itself the first time someone checks it; a correct rule applied
incompletely does not, because every spot-check confirms the rule.

The general lesson for the next inventory: **derive the member list mechanically from the criterion** —
`scripts/callers.sh run_path` would have listed `mc_harness.rs:281` on day one — rather than writing
down the members you can think of and naming the criterion afterwards.

Re-emitted under D6.b: 119 rows before and after, **1** changed, 0 namespaces, 118 byte-identical, and
exactly one row red before any sha moved. Gated by `theta_surface_reproduction.rs::mc_reproduces_anchor`
(**measured 256 s**), and green.

With it, **every scenario the `backtest`/`param_robustness_sweep`/`monte_carlo` binaries can produce
now has a re-run gate.** What remains ungated is the 15 forecast/report-binary scenarios, a different
producer family entirely.

### `#127` — the clamp evaluates the ruin predicate three times in order to hide its consequence, and records it zero times
**Status**: disclosed 2026-09-27, owned by story 1-28 (`ready-for-dev`). Anchor-impacting: **yes, when
fixed** — new columns change the rendered body; the re-emission protocol is an entry gate, not dev's call.

Every Monte-Carlo equity value `e <= 0` is mapped to `dec!(0.000001)` before the metric calls, at three
independent sites: `crates/backtest/src/bin/param_robustness_sweep.rs:911-917`, the same file again at
`:1974-1980`, and `crates/backtest/src/mc_harness.rs:294-303`.

**The clamp is correct and is staying.** It exists so `compute_sharpe_hourly` cannot return NaN on a
ruined path (ADR-0051 D2 asserts NaN absent), and removing it would move every Sharpe, Sortino, Calmar
and max-drawdown number in the corpus. Nothing here argues against the clamp.

The defect is that the predicate `e <= Decimal::ZERO` is **evaluated in order to substitute a value, and
the answer is then discarded**. After the substitution, a path that lost everything and a path that lost
99.9 % are the same number. `p95_maxdd = 100.00 %` is the signature of ruin and is also what a merely
catastrophic path prints.

#### Why this is the week's shape moved one layer out

`#102`, `#104`, `#112`–`#126` were all *gates that could not fail*. This is an **observable that cannot
be observed**, and it is harder to notice for one specific reason: a broken gate at least prints a verdict
you can distrust, whereas this prints a perfectly ordinary percentage. There is no tell in the output. The
information is destroyed one line before the number that needed it.

**A substitution is a lossy write.** Where code replaces a value because the real one is unusable, the
replacement site is exactly where the discarded fact has to be recorded — it is the only place that still
knows.

#### The triplication is the multiplier, not a style complaint

One clamp is one place to remember the counter. Three clamps are three places to forget it, and the third
copy was added without anyone noticing the first two had the same gap. Story 1-28 AC1 therefore factors
one helper returning the clamped curve **and** the witness, with a test asserting the literal
`dec!(0.000001)` occurs exactly once in `crates/backtest/src/` — so a fourth copy goes red.

#### Measured before scoping: this is a tripwire, not a correction

Across `evidence/**/reports/*theta-surface*.md`, the **live** (2026-09-25, post-re-lock) bodies contain
**zero** cells at `p95_maxdd >= 99.9 %`. Cells at exactly `100.00 %` survive only in two **superseded**
2026-06-08 `mn-basisperp` bodies — which is precisely where the clamp was hiding ruin, and is the
retro-validation of the concern. So the new columns will render `0` everywhere on today's evidence.
Nothing in the live corpus is being misread, and the guard lands before the next surface that would be.

The second half of story 1-28 — `trades` incremented for real fills at
`crates/backtest/src/scenarios/montecarlo.rs:620` and for synthetic maintenance-margin cover legs at
`:839` — is **not** a new finding: `#110` disclosed it and 1-21 shipped the honest interim, a legend at
`sweep_harness.rs:2345-2348` that states the conflation rather than hiding it. It is carried here only
because both halves need the same threading.

#### Records clarification (not a defect): what "re-emitted in place" means

The D6.b records for 1-26 / 1-21 / 1-27 say the bodies were re-emitted **in place**. That means *the same
directory and the same anchor namespace* — **not** the same filename. The re-lock wrote new timestamped
files beside the old ones (`robustness-sweep-20260925-*` next to `robustness-sweep-20260608-*`), and
`scripts/verify_anchors.sh` resolving each anchor to the **newest** match is what makes the 2026-09-25
bodies live and the 2026-06-08 bodies history-on-disk. Recorded because the orchestrator misread it as
same-file on 2026-09-27, and because the measurement above is only interpretable once you know which
bodies are live.

#### One suspicion checked and refuted rather than filed

`scripts/callers.sh liquidations` reported a single reader, inside a test — which would have made the
rendered MN `liquidations` column structurally always-zero, the same defect a third time. It is fully
wired (`param_robustness_sweep.rs:908` → `:1879` sum → `:1913` → `sweep_harness.rs:2468`) and
demonstrably rendered 2210 events before the `#71` fix removed the absorbing state. Two greps, no
finding. Noted because `callers.sh` matches whole words and `total_liquidations` is a different
identifier — on a struct **field**, its count is a lower bound in a way its function-symbol output is not.

### `#128` — the forecast/report binaries default their WRITES into the anchored corpus and hardcode their READS to dated paths inside it
**Status**: disclosed 2026-09-27. Anchor-impacting: **(a) can destroy an anchored body outright.**
Derived by a delegated read-only agent (`docs/dev-notes/forecast-bin-repro-recipes-2026-09-27.md`,
686 lines, no binary run); the five load-bearing claims below were each re-verified by the
orchestrator at the cited lines before being filed.

The 15 scenarios left ungated after `#126` are a different producer family — six binaries in
`crates/forecast` and `crates/backtest`, not the `backtest`/`param_robustness_sweep`/`monte_carlo`
family. Deriving how to reproduce them surfaced five defects with one theme: **these binaries treat
the byte-immutable evidence corpus as their working directory.**

#### (a) A no-argument run OVERWRITES a byte-immutable anchored body

`crates/backtest/src/bin/threshold_sweep.rs` combines two things that are each defensible alone:

```rust
//  :152   #[arg(long, default_value = "evidence/v1/v25-tcn-threshold-tuning/reports/")]
//         out_dir: PathBuf,
//  :1045  let report_filename = format!("threshold-sweep-{label}-realdata-recalibrated-20260521.md");
```

A **hardcoded date** in the filename plus a **default out-dir pointing into the corpus**. Every other
bin in this family embeds *today's* date, so a careless run merely plants a newer file that silently
becomes what `verify_anchors.sh` hashes — bad, and recoverable by deleting it. This one writes over
the anchored body itself. Recoverable only from git, and only if someone notices.

So for this bin `--out-dir <tempdir>` is not advisory, and the mandatory-`--out-dir` note in the 1-26
record was about a different binary.

#### (b) Nine zeros labelled "graceful degradation", read from a path the re-lock pattern moves

The same bin reads its predecessor's numbers from **hardcoded, dated paths inside the corpus**
(`:106`, `:109` → `evidence/v1/v25-tcn-recalibrate/reports/forecast-distribution-bs{1,2}-realdata-recalibrated-20260521.md`),
consumed at `:741`. On any read failure `parse_gate_survivors` (`:246-251`) returns `[0usize; 9]`, and
its own doc comment calls that "graceful degradation". No error, no log. Nine zeros then render in the
report indistinguishably from nine measured zeros.

**This is armed, not hypothetical.** Those two predecessor rows are on the list of scenarios awaiting a
re-run gate, and this project's re-emission pattern writes a **new timestamped filename beside the old
one** (see `#127`'s records clarification). Re-emit them and the anchor resolver follows the new body
while `threshold_sweep` keeps reading the 2026-05-21 one — the gate would validate one body while the
downstream report quotes another, silently. Delete the old one instead and the parser returns zeros.
Neither failure announces itself.

#### (c) A refusal is reported as a convergence failure

`crates/forecast/src/bin/regime_verdict.rs:837-853` handles a refused child correctly as far as
flagging goes — `completed = status.success()` is carried onward — but when no report is found it warns
and substitutes `String::new()`, and the verdict it then emits is **V-REG-1, defined at `:197` as "EM
convergence failure"**. Exit 0. So a build without the right features produces a report asserting that
the EM algorithm failed to converge, when what actually happened is that the backtest binary never ran.

This is the `#114` / `#124` shape again, and the reason it keeps earning an entry: **a diagnosis that
names the wrong cause is worse than no diagnosis, because it aims the next reader away from the
answer.** Three of the six bins declare `required-features` and refuse at cargo target-selection;
`vol_verdict`, `regime_verdict` and `sharpe_comparison` declare none, and of those only
`sharpe_comparison` bails (`:414-416`).

#### (d) The `candle` feature is decoupled from whether candle is linked

`crates/backtest/Cargo.toml:38` declares `candle = ["strategy/forecast"]`, while `:63` already sets
`strategy = { path = "../strategy", features = ["forecast"] }` **unconditionally**. The feature
therefore adds nothing to the dependency graph; its only effect is flipping `#[cfg]` blocks in
`scenarios/*.rs` and the `threshold_sweep` target gate. The comment above it (`:36-37`) is true about
those cfg blocks and false as a statement about the build: reasoning "candle is not linked, so the
tensor path cannot run" is wrong in both directions.

#### (e) Two rows are not reproducible from a tempdir at all

`recalibrate_sigma_train.rs:455-459` writes the **resolved overlay path** into `body`
(`"- Read-only against \`{overlay_path}\` original safetensors."`, fed from `:686`), and
`scripts/hash_report.py` strips only the YAML front-matter before hashing. So the anchored SHA of
`recalibrate-sigma-train-bs1` / `-bs2` **encodes an absolute-ish input path**, and pointing
`--anchor-dir` at a tempdir changes the digest by construction. A reproduction gate for these two
either asserts against the committed checkpoint directory (`git diff --exit-code
crates/forecast/checkpoints/anchors/`) or the body stops printing the path — and the latter is itself
a body change, so a D6.b re-emission.

A hashed body should contain what the run MEASURED, not where the run FOUND it. An input path is
provenance and belongs in the front-matter, which is excluded from the hash for exactly this reason.

#### Consequence for the plan

12 of the 14 gateable rows are measurable today (the 15th, `sharpe-comparison-realdata`, has no
producer at HEAD — `#118`). Total compute ≈ 79 min, taken from the anchored reports' own
`wall_clock_s` front-matter rather than estimated, ranging from `vol_verdict` at 0.7 s to
`recalibrate_sigma_train --scenario bs2` at 619.8 s. Every required checkpoint and corpus file is
present on this machine. Three rows are *predicted* to drift because their backtest inputs were
re-emitted after they were locked — that prediction is dated inference, not measurement, and a red
there is a D6.b candidate rather than a gate bug.

### `#129` — the `#67` guard turned a mispriced fill into a silently dropped order, on an anchored lane the fix never reached
**Status**: disclosed 2026-09-27. **CRITICAL.** Anchor-impacting: 2 rows
(`threshold-sweep-bs{1,2}-realdata-recalibrated`, `anchors.toml:262` / `:267`). Found by a delegated
read-only audit of story 1-25's acceptance criteria; the orchestrator re-verified every claim below at
the cited lines.

`run_cell` is **not** in `crates/backtest/src/bin/threshold_sweep.rs` — it is
`crates/backtest/src/scenarios/threshold_sweep.rs:56`, and the bin's `fn @643` is its caller
(`:783`, `:845`, `:937`). The orchestrator grepped the bin, found nothing, and treated the absence as
weak evidence the guard might be missing. `scripts/callers.sh run_cell` answers it in one call. I had
written that instruction into the audit's own brief and then not followed it myself.

#### The source has confessed since v0.1.1

`crates/backtest/src/scenarios/montecarlo.rs:5-9`, verbatim at HEAD:

> *from v0.1.1 `run_path` carries the Bug-B long-only solvency guard (pre-flight cash check +
> fill-loop guard), which `run_cell` does NOT — `run_cell` retains the pre-Bug-B unguarded Buy sizing
> inside the frozen-anchored threshold-sweep lane.*

Confirmed by reading it: `equity = cash + position_value` (`scenarios/threshold_sweep.rs:195`), the only
cash-adjacent test is `if equity <= Decimal::ZERO { continue }` (`:196`) — which does not bound **cash**
— and Buy sizing is `equity * dec!(0.10)` (`:207-208`) with no cash comparison anywhere. So story 1-25's
AC2 ("BOTH lanes fixed") is unmet, and has been unmet in writing the whole time.

#### The part nobody wrote down

`run_cell` also never received the per-symbol routing half of the `#67` fix, and both of its engine
calls are written as:

```rust
//  scenarios/threshold_sweep.rs:228 and :269
&& let Ok(fills) = engine.step(bar, vec![ord]).await
{ … }
```

An `if`-chain with **no else arm**. Since the `#67` engine guard makes `PaperEngine::step` return
`Err(MatchError::SymbolMismatch)` for a cross-symbol order, the pattern simply fails to match and the
entire body is skipped. No log, no counter, no error, no fill.

**So the `#67` fix did not leave this lane unfixed — it changed its behaviour.** Before the guard, a
cross-symbol order was priced at the wrong symbol's bar: wrong, loud in the arithmetic, and the defect
`#67` was filed for. After the guard, the same order **vanishes**. A dropped order and a mispriced fill
are different wrong answers, and only one of them is invisible.

That behaviour change landed on a lane with two frozen anchors and was never disclosed — which is the
part that makes this CRITICAL rather than merely open. `#67`'s inventory was scoped to the lanes it
believed the fix touched; this is a third way that scoping failed, after `#111` (scoped by lane while
the fix landed beneath the lanes) and `#126` (the criterion was right, the enumeration under it was
incomplete). Here the lane was **known** to be divergent, said so in its own module doc, and still was
not re-examined when the engine beneath it changed.

#### The compounding defect, not yet manifest

No solvency guard → cash can go negative → final equity can go negative → `compute_calmar`'s unguarded
`powf` → **NaN into a hashed body**. `bin/threshold_sweep.rs` applies no clamp at all, calling
`compute_calmar` raw at `:790` and `:945` — unlike every other lane, which clamps (see `#127`).
Measured: `grep -rl NaN evidence/*/reports/` is empty, so it has not happened. The guard against it is
arithmetic luck, not code.

#### Direct collision with story 1-29

`threshold-sweep-bs{1,2}-realdata-recalibrated` are 2 of the 12 rows story 1-29 sets out to gate. A
reproduction gate on them would pin, as correct-by-definition, a body produced by a lane that silently
drops orders. **1-29 must not gate those two rows until this is ruled on** — otherwise the gate
converts an undisclosed behaviour change into a defended invariant, which is the most expensive form of
this project's recurring mistake.

The ruling is the operator's, and the honest options are: fix `run_cell` (routing + solvency guard) and
re-lock its two anchors under D6.b; or narrow 1-25's AC2 to `run_path` explicitly **and** disclose the
silent-drop behaviour change in the story record and in the two anchors' comments. Narrowing the AC
without the disclosure is the one option that is not available.

**2026-09-27 (orchestrator) — `#118` archaeology: the framing was wrong on both cases, and they are not
one shape.** A delegated read-only history audit (`docs/dev-notes/118-anchor-archaeology-2026-09-27.md`)
settled both. The two cases share only "the gate passes on a body nothing can regenerate" — and that
much is still a valid **byte-immutability** check. What is false on these rows is the *reproduction*
claim, not the measurement.

**`sharpe-comparison-realdata` (2 rows = ONE body pinned twice, the second a v5 namespace copy carrying
the identical SHA).** The producer was renamed to `sharpe-comparison-patchtst-bs1-realdata` by
`bd731338` (2026-05-22) — but the same commit grew the scenario set 4 → 5 and restructured the verdict
table, so the anchored 4-row body is not a body the surviving name can ever emit. That **forces
option (b) out**: re-keying would pin a digest the surviving name has never produced. The evidence does
not force a choice between relabelling and documenting the gap. Its numbers are not uniquely cited
anywhere live.

**`eth-yahoo-2024-1d-sma-cross` — `#118` said "re-emitting is not available", and that is wrong.** The
row was *knowingly* left on a pre-migration body by `e74204a9` (2026-05-28, whose own message says
"UNCHANGED … not re-emitted at v0.1.3 per analyst defer"), the replacement digest was measured, and it
is **already committed**: `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs:156`,
`ETH_ANCHOR_SHA = "c854ff2b…"`, with the reason spelled out at `:149-156` and deferred to a v0.1.4 ship.
**That ship was retired** (`trace.toml:2270`, operator decision 2026-06-16) and its retirement note
reasons only about the 9 *new* tickers, silent on the 1 *in-place* row it also owned — while
`trace.toml:2269` still reads `anchors = []  # tester M-FINAL fills with row 70`. The producer still
runs at HEAD. So there is a fourth option `#118` did not list: **discharge the 2026-05-28 defer.** Its
numbers ARE load-bearing — `anchors.toml:664` rests the H1 hypothesis discharge on this body's `+2.76%`.

Mechanism, which narrows the option space: **no `historical` field exists** in `anchors.toml` or any
reader, so a real relabel needs a `verify_anchors.sh` change. Relabelling by deleting the `sha256` line
is no longer possible — `#127`'s hardening fails with *"N row(s) were never compared"*. Deletion has an
existing ratification hook (`MIN_EXPECTED_ANCHORS`), but **ADR-0038 § D6.b forbids row deletion
outright** and already names `D6.c` as the slot for a protocol amendment; no `D6.c` exists.
`scripts/spec_lint.py:601`/`:608` cross-check trace ↔ anchors both ways, so any re-key edits
`trace.toml` in the same pass.

Two corrections to `#118`'s own text: it cited `retired-surface-inventory-2026-05-22.md:157` as evidence
the surface is retired — that line asserts the opposite (that `sharpe_comparison.rs` still emits the
name) and was **already false when written**, 23 h after the rename landed. And grouping the two cases
as "the same shape" does not survive the history.

### `#130` — every report this producer emits claims it was generated on 2026-05-21, forever
**Status**: disclosed 2026-09-27. Anchor-impacting: **no** — and that is the finding, not a mitigation.

`crates/backtest/src/bin/threshold_sweep.rs:1006-1028` builds the `generated:` front-matter field like
this, verbatim at HEAD:

```rust
let secs = now;                        //  real epoch seconds
let mins   = secs / 60 % 60;           //  real
let hours  = secs / 3600 % 24;         //  real
let _days_since_epoch = secs / 86400;  //  computed, then DISCARDED — note the underscore
// Approximate date from epoch days (good enough for advisory frontmatter).
format!("{}-{}-{}T{:02}:{:02}:{:02}Z", 2026, "05", "21", hours, mins, secs % 60)
```

The year, month and day are literals — and they are not arbitrary literals, they are **the date the
anchored bodies were born**. Only the time of day is live. So a run today emits
`generated: 2026-05-21T21:38:08Z`: a real clock time welded to a four-month-old date. The comment
promises an approximation from epoch days and the very next line throws that number away.

**Why it survived, and why that is the point.** `generated:` lives in the YAML front matter, and
`scripts/hash_report.py` strips the front matter before hashing — by design, so that re-emitting a
body does not churn the digest over a timestamp. So this field is the one part of the report that
**nothing checks**: not the anchor gate, not the determinism literals, not spec-lint. It drifted into
fiction precisely where no gate was looking, and it will keep asserting May 2026 until someone reads it
next to a `git_commit` from September.

That combination is what makes it worth an entry rather than a silent fix: the body carries a real
`git_commit` and a real `wall_clock_s` beside a fabricated date, so the front matter is internally
inconsistent in a way that misdates evidence. The repo's own convention elsewhere is a real UTC stamp
in the FILENAME (`backtest-20260927-123805-…`), which is how the mismatch is visible at all.

Isolated to this one producer — `grep` for the literal and for the comment finds no other site.

Found while measuring `#129`: the freshly emitted body's `generated:` line read 2026-05-21 with today's
time, which is what prompted reading the construction rather than trusting it.

**2026-09-27 (orchestrator) — `#129` MEASURED in three states; the fix is landed, the re-lock is NOT.**
Full record with every digest: `docs/dev-notes/129-threshold-sweep-measurement-2026-09-27.md`.

Six distinct body digests for two scenarios — ANCHOR (2026-05-21, `447c0432`), PRE-FIX (HEAD with
`run_cell` untouched) and POST-FIX. **`PRE-FIX ≠ ANCHOR` on both**, so this lane had already stopped
reproducing before today, while `verify_anchors.sh` said 119/119 throughout. That makes it the THIRD
such family after 1-26's 34 θ-surfaces and 1-27's 15 rows, and it sat outside both inventories. Its
cause is **not bisected**, and must not be assumed to be `#67` — `#125` is the precedent for a drift
that resembled `#67`'s signature and had a different cause.

**The verdict had already flipped before the fix:** `T-MARGINAL` → `T-ALPHA-UNLOCKED`, at PRE-FIX
already. Composition matters more than the label: of the 1.8338 by which `max_cell_sharpe − v1_sharpe`
grew, **0.6963 (38 %) is the CONTROL getting worse**, not the strategy getting better. The FROZEN gate
is uninvolved (grepped — neither file references `classify_verdict`/`verdict_bands`/
`compute_robustness_flag`/`rank_candidates`); `T-*` is this report's private vocabulary over a 9×5 grid
with no bootstrap, no DSR and no multiple-testing correction, selecting a maximum from 45 cells.

**What I could not establish, stated rather than glossed:** `POST-FIX ≠ PRE-FIX`, but not *which* half
of the fix moved it. The two new branches log through `tracing::warn!` and the run emitted **zero log
lines of any level** — no subscriber is installed in this bin and `RUST_LOG` was unset. I first read
"0 refusals" as proof the silent-drop path was never taken; it proves nothing, and the post-fix code
*cannot* log a refusal because it never hands a foreign bar to the engine. An absence of output from a
logger that was never installed is this week's own subject matter, and I nearly filed it as evidence.
The report renders no order or trade counts either, so the artefact cannot answer it — `#110` added a
`trades` column to the MN surfaces for exactly this reason and this family never got one.

**Landed:** the code (ratified `#67` seam — `last_bar_by_symbol`, the fill-side twin of `mark_prices`,
mirroring `montecarlo::run_path` — plus the Bug-B pre-flight, and a reported refusal in place of a
silent `if let` drop). 282 lib + 15 integration tests green, clippy clean on a forced re-lint.
**Not landed:** the anchor re-pin. Three independent reasons — AD-19 (a thesis-adjacent verdict flip
escalates BEFORE publication, and the ruling was given before the flip was known); re-pinning a number
whose cause is unnamed is re-baselining (`#77`); and the old verdict is cited in `trace.toml:372`,
**accepted ADR-0036:45**, and inside another `evidence/` body
(`v25-tcn-horizon-bump-or-retire/reports/test-final-2026-05-21.md:75`) that cannot be edited to match
without breaking its own bytes — the `#110` knot, not dev's to untie.

**2026-09-28 (orchestrator) — `#118` case 2 DISCHARGED: `eth-yahoo-2024-1d-sma-cross` re-emitted under § D6.b.**
Operator ruling 2026-09-27: re-emit eth, document the sharpe gap. Both done.

The 2026-05-28 defer is discharged rather than inherited. Measured 2026-09-27T21:59:28Z from the
workspace root, `target/release/run_yahoo_sma --ticker ETH-USD --reports-dir <tempdir>`,
`--features yahoo`, release: the run reproduces **`c854ff2b…` exactly** — byte-for-byte the digest
measured at the v0.1.3 migration four months ago and committed since as `ETH_ANCHOR_SHA`
(`crates/backtest/tests/run_yahoo_sma_ticker_flag.rs:156`). Four months and a great many engine commits
apart, the same bytes. This lane is **single-symbol**, so `#67` never touched it — which is why it held
while the multi-symbol lanes did not, and is the mechanism agreeing with itself rather than a
coincidence.

Landing control, proven not assumed: with the new body on disk and **before** the sha was touched, the
gate showed exactly **1 FAIL / 118 PASS**, and the single FAIL was this row resolving to the new file
with exactly the digest that then replaced it. After: `ANCHORS PASS (119 / 119) [declared 119 · floor
119]`, and `git diff` on `anchors.toml` shows **exactly one** `sha256` line and one `version` line
changed — 118 rows byte-identical. The superseded body stays on disk, linked from the row's comment
(§ D6.b forbids row deletion). Its `+2.76 %` total return, which carries the H1 hypothesis discharge,
is **unchanged** by the re-emission, so that discharge still stands.

One limit stated rather than hidden: `data/yahoo/ETH-USD/1d/2024/` is gitignored and operator-local, so
**CI cannot reproduce this row**. The three Binance corpora were committed the same day for exactly this
reason; the Yahoo cache was not part of that decision.

### `#131` — the reconciler's own SCOPE was the unstated assumption: "29 of 29" was true, and silent about four more files
**Status**: FIXED 2026-09-28. Anchor-impacting: **no** — 0 mismatches found, which is itself the finding.

`scripts/check_determinism_anchors.py` exists to stop a second SHA system drifting from
`evidence/anchors.toml`. It printed `OK — 29 of 29 resolved literal(s) match … Sites seen: 29 (floor
29)`, every word of which was true about `crates/backtest/tests/determinism.rs` — the only file its
`main()` ever opened.

Derived mechanically rather than listed from memory
(`grep -rln 'const [A-Z_]*ANCHOR[A-Z_]*\s*:\s*&str' crates/ --include='*.rs'`), the repo holds **35**
such literals across **five** test files. Six of them — in `multi_pair_determinism.rs`,
`run_yahoo_sma_ticker_flag.rs`, `theta_surface_reproduction.rs` and `reproducibility_sample_figure.rs`
— were reconciled by nothing. `076929bb63d9bec0…` (the BTC yahoo anchor) is duplicated in **three**
places with nothing comparing them.

So this is the tool's own subject matter, one level up. A gate that reports a count it did measure can
still mislead, if the SET it measured is narrower than the set the reader assumes — and the assumption
lived in a single module-level constant. The summary now prints a **per-file breakdown**, so a file
dropping out of scope is visible in the output rather than only in a total that still looks plausible.

**All six already matched.** Report a clean result as a result: those gates were correct, they were
merely unguarded — had one been wrong it would have been wrong silently for as long as the file
existed. Five of the six are additionally live-reproduced by tests that RAN rather than skipped.

Four defects surfaced while extending it, each fixed and each the same family:

- **File-scope consts inherited an unrelated function's `#[cfg(feature = …)]`** because `current_fn` is
  sticky, so on the first broadening the two literals the whole change exists to check were classified
  "skipped: cfg-gated" — dropped at the last step and counted as fine. Sites at brace depth 0 are now
  `(file scope)` and never cfg-inherited.
- **Declaration bleed between adjacent file-scope consts**: the backwards window stopped only at `fn`,
  so with no `fn` between two consts, `ETH_ANCHOR_SHA` inherited BTC's declaration and resolved to the
  wrong scenario. It now also stops at the previous anchor const. This is the *second* time a
  too-forgiving window laundered one site's meaning into another in this same file.
- **The broadened name regex could not match a bare `ANCHOR`**, zeroing all 29 original sites — caught
  instantly because `--self-test` probes stopped firing. The non-vacuity machinery earned its keep on
  its own author.
- **Documentation drift**: `ETH_ANCHOR_SHA`'s doc comment still claimed row 70 held the superseded
  digest and that re-emission was "deferred to the v0.1.4 BNB ship" — false since that row was
  re-emitted, and the ship was retired 2026-06-16.

Tool now reports `35 of 35 … across 5 file(s) (floor 35)` with the per-file split; `--self-test` runs 9
probes. Probed independently by the orchestrator after the fact: mutating one digit of
`reproducibility_sample_figure.rs`'s literal makes it FAIL naming that file and line and printing both
digests; restored, green. Record: `docs/dev-notes/131-anchor-literal-reconciler-scope-2026-09-28.md`.

### `#132` — the threshold-sweep anchor was broken by a directory rename, because the body records where the run looked
**Status**: first divergence FOUND 2026-09-28 by bisect (story 1-30 AC1). Anchor-impacting: yes — this is
the first of at least two steps; the later, arithmetic one is still unattributed (1-30 AC2).

`git bisect`, 766 commits, **no path restriction** (`#111` is the standing warning that scoping by the
lanes you believe are involved is how `#67` hid for six weeks), GOOD = `42e084e0`, BAD = HEAD, ten steps.

**First bad commit: `13955206` (2026-06-28) — `refactor(spec): reorganize into spec/v1 (implemented) +
spec/v2 (research-driven)`.** A documentation reorganisation. Its entire diff against
`crates/backtest/src/bin/threshold_sweep.rs` is **seven `//!` doc-comment lines**, and a doc comment
cannot move arithmetic.

The mechanism is line 98 of the emitted body:

```
ANCHOR    (Read from predecessor `spec/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md` body — NOT re-computed.)
TODAY     (Read from predecessor `evidence/v1/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md` body — NOT re-computed.)
```

**The hashed body records where the run found its input.** `spec/…` → `spec/v1/…` on 2026-06-28, then
`spec/…` → `evidence/…` at the BMAD migration: each rename rewrote that sentence and therefore the
digest, with **zero numbers changed**. This is `#128e` exactly — there, `recalibrate_sigma_train` welds
its overlay path into the body, making two rows unreproducible from a tempdir. Two producers, one
defect: **a hashed body should carry what the run MEASURED, not where it LOOKED.** A path is provenance
and belongs in the front matter, which is excluded from the hash for precisely this reason.

#### The hypothesis I had, and why it was wrong

I expected `#128b` — the hardcoded dated predecessor path at `bin/threshold_sweep.rs:106/:109` being
moved out from under the reader, whose `parse_gate_survivors` then returns nine zeros it calls "graceful
degradation". Checked instead of assumed, and it did not happen: the literal and the file moved together
in that same commit (`spec/v25-tcn-recalibrate/…` → `spec/v1/v25-tcn-recalibrate/…`, both present at
their respective commits), and the Gate-survivor counts are **identical** in the anchored and the
current body — 69085, 60339, 51964, 44375, 37386, 31177, 25973, 21684, 18087. `#128b` stays armed and
unfired.

#### What this does and does not settle

It settles **when byte-reproduction broke**: 2026-06-28, by a rename. It does **not** explain the
numbers. The post-reorg digest is `35e3ce7d…` and today's pre-fix digest is `1dc4855f…`, so at least one
more step exists, and that one IS arithmetic — v1 Sharpe moved `+0.003098` → `−0.328302` somewhere
between.

**Instrument note for 1-30 AC2, learned here:** do not bisect the second step on the body SHA. That
digest is polluted by provenance strings — the BMAD migration renames this path a second time and will
produce another pure-path step. **Bisect on a NUMBER instead** (v1 Sharpe is the cleanest: single value,
present in every version of the body, and it demonstrably moved). Bisecting a hash answers "did any byte
change"; bisecting a number answers the question actually being asked.

#### Retro

The bisect cost ~2 h of compute and returned a documentation commit. That is the right answer, not a
wasted run — and it would have been unavailable to any amount of reading, because nothing in the diff
of `13955206` suggests it could move a backtest. The corpus trap was real and pre-empted: the 720
parquets became tracked on 2026-09-28, so every checkout before that deletes them from the working
tree; the step script restores them from a copy outside the repo before each build.

**2026-09-29 — `#129`/`#132` AC2: the arithmetic step is `11acd126`, the `#67` fix itself.**

Second bisect, instrument **v1 Sharpe** rather than the body SHA (bug-log `#132` explains why: the
digest is polluted by provenance strings and the BMAD migration renames that path a second time, so a
hash bisect would only have found another pure-path step). GOOD `42e084e0`, BAD HEAD, eleven steps, two
`SKIP-build` commits handled as untestable rather than as divergences.

**First bad commit: `11acd126` (2026-08-16) — `fix(1-25,#67): … engine guard + per-symbol fill
routing`.** v1 Sharpe `+0.003098` → `−0.328302` exactly there.

So the lane's history is now fully attributed, in two steps with two different kinds of cause:

| date | commit | what moved |
|---|---|---|
| 2026-06-28 | `13955206` (doc reorg) | the **bytes** only — the body prints its predecessor's path |
| 2026-08-16 | `11acd126` (the `#67` fix) | the **numbers** |

**This confirms `#129` by measurement rather than by argument.** Yesterday the record could only say
`POST-FIX ≠ PRE-FIX` without separating the repair's two halves, because the bin installs no tracing
subscriber and the report renders no order counts. The bisect answers it from the other end: the
arithmetic moved at exactly the commit that introduced the engine guard, and `run_cell` never received
the routing half — so its `&& let Ok(fills) = engine.step(…)` chain, having no else arm, began dropping
cross-symbol orders in silence from that day. The mechanism predicted the commit and the measurement
found it.

#### Which of the three states is the correct one — and it is none of the two that are pinned

| state | v1 Sharpe | best cell | verdict | what its arithmetic is |
|---|---|---|---|---|
| ANCHOR (pinned) | `+0.003098` | `+0.018254` | `T-MARGINAL` | pre-`#67`: cross-symbol fills **mispriced** |
| PRE-FIX | `−0.328302` | `+0.354852` | `T-ALPHA-UNLOCKED` | post-guard: those orders **silently dropped** |
| POST-FIX | `−0.693194` | `+1.155769` | `T-ALPHA-UNLOCKED` | routed correctly + solvency guard |

The anchored body is `#67`-contaminated; the state the corpus silently drifted into is drop-contaminated;
only today's code is correct. A re-lock therefore pins the POST-FIX bodies (`04564535…`, `f7008296…`),
not the pre-fix ones — the same conclusion 1-27 reached for its 15 rows, arrived at independently here.

#### The verdict flip survives the correction, and that is the thing to escalate

`T-ALPHA-UNLOCKED` is not an artefact of the silent drops: under **corrected** arithmetic the delta is
larger still (`1.155769 − (−0.693194) = 1.8490` against a `+0.10` line). What it means is the part that
must not be lost in the label: the v1 baseline **loses money** (`−0.693194` Sharpe, `−27.52 %` total
return), and the report selects the **maximum of 45 cells** with no bootstrap, no DSR and no
multiple-testing correction. Beating a losing baseline, chosen as the best of 45, is not an edge, and
the FROZEN robustness gate — which is what this project means by an edge — is not involved anywhere in
this report (grepped: neither file references `classify_verdict`, `verdict_bands`,
`compute_robustness_flag` or `rank_candidates`). The era-qualified thesis is untouched by a point
estimate that never faced that gate.

**2026-09-29 (orchestrator) — CORRECTION to my own `#129`/`#132` numbers: I read deltas as absolute
Sharpes.** The threshold-sweep report has **no absolute-Sharpe heatmap at all**. Heatmap A is titled
*"Sharpe (ann.) delta vs v1 momentum"*, and every figure I quoted from it as a "best cell Sharpe" is a
**delta**. The corrected picture:

| state | v1 Sharpe | delta (Heatmap A) | best cell, ABSOLUTE |
|---|---|---|---|
| ANCHOR (2026-05, `#67`-contaminated) | `+0.003098` | `+0.018254` | **`+0.021352`** |
| drifted (post-guard, silent drops) | `−0.328302` | `+0.354852` | **`+0.026550`** |
| re-emitted (corrected routing) | `−0.693194` | `+1.155769` | **`+0.462575`** |

So the corrected arithmetic does lift the best cell substantially — `+0.021` → `+0.463` — but it never
approached the `1.16` I wrote, and the headline that reads best is the pair: **the best cell rises to
+0.46 while the baseline falls to −0.69**, and the delta is the sum of those two movements.

Where the wrong figures stand: the `#129`/`#132` AC2 entry above, the 2026-09-27 measurement note, and
story 1-30's board line, all of which say "best cell" where they mean "delta". They are left as written
with this correction as the record, per the project's convention of not silently rewriting a filed
finding.

**What caught it:** the operator-ruled verdict qualifier, on its very first render. It prints
`best cell {headline.sharpe} − v1 baseline {v1_sharpe} = {max_sharpe_delta}` computed from the run, so
the two quantities appeared side by side and the misreading had nowhere to hide. A qualifier written
down as fixed prose would have repeated my error instead of exposing it — which is the argument for
computing such a line rather than authoring it, and it earned its keep before the body it explains had
even landed.

**2026-09-29 (orchestrator) — CORRECTION: `threshold_sweep` DOES install a tracing subscriber.**
I recorded three times — in the `#129` measurement note, in the `#129`/`#132` bug-log entries and in
story 1-30 — that this binary installs none, and used the run's zero log lines as the evidence. Both
halves were wrong, and the second is the instructive one.

The subscriber is installed at `crates/backtest/src/bin/threshold_sweep.rs:700`
(`llm::tracing_init::install_global(&[], false).ok()`). The reason nothing appeared is one line
inside it: `EnvFilter::from_default_env()`, which with `RUST_LOG` unset admits **ERROR only**. Every
`tracing::warn!` in the lane was filtered, not absent.

So the diagnosis I filed — "an absence of output from a logger that was never installed" — named the
wrong cause while being right that the absence proved nothing. That is the `#114`/`#124` shape applied
to my own note: **a diagnosis that names the wrong cause aims the next reader away from the answer**,
and here the wrong cause was expensive in a specific way — it made AC5 look like a build (install a
subscriber, thread it through) when the actual gap was one environment variable and a counter.

Consequence for story 1-30 AC3/AC5: no new plumbing is needed for the logging half. What IS needed is
a counter, because even with `RUST_LOG=warn` today's repaired code cannot log a refusal — it never
hands a foreign bar to the engine, so its refusal branch is unreachable by construction. The set that
matters is *orders routed to their own symbol's bar rather than the merged-loop bar*: exactly what the
pre-repair code handed over with a foreign bar and, after the `#67` guard, dropped in silence. That
counter plus the Bug-B pre-flight counter now log once per cell at WARN, and they are **logged, not
rendered** — so AC3 is answerable without a third re-emission of bodies re-locked hours ago.

**2026-09-29 — story 1-30 AC3 ANSWERED by measurement: the movement is the routing half, not the
solvency guard.** The `#129` repair had two behavioural halves and yesterday's record could only say
`POST-FIX ≠ PRE-FIX` without separating them. Measured now, 47 cells of the bs1 sweep, one diagnostic
line per cell at WARN with `RUST_LOG=warn`:

| counter | total | per cell |
|---|---|---|
| `cross_symbol_routed` | **148 719** | min 1 900 · max 5 064 |
| `solvency_skips` | **6** | 44 of 47 cells fired it zero times |
| fills | 183 751 | |

**80.9 % of every fill in this lane is cross-symbol.** Each of those, before the repair, was handed to
`engine.step` with the merged-loop bar instead of the order's own — mispriced before `#67`, silently
dropped after it. The Bug-B solvency pre-flight fired **6 times in 183 751 fills (0.003 %)**, so it
contributes essentially nothing to the numbers. AC3 asked which half moved the result; the answer is
the routing half, and it is not close.

**Proven body-neutral rather than assumed:** the diagnostic run re-emitted the bs1 body at
`924a51bb…` — byte-identical to the row re-locked hours earlier. The counters are logged and never
reach a `body.push_str` path (grepped: 0 hits), so the reasoning holds for the later message-wording
fix too.

**Decision (orchestrator, recorded not asked): the counters stay logged and are NOT rendered.** The
report already renders `Trades (cell)` for the headline cell. A per-cell `cross_symbol_routed` column
would save a future reader one command — against a third re-emission of two bodies re-locked the same
week, and against the churn to every downstream citation that entails. The diagnostic is one env var
and is written down here with its exact invocation. If the lane is re-emitted again for an independent
reason, the columns ride along then. Reverse this in one sentence if you disagree.

Reproduce:
`RUST_LOG=warn ./target/release/threshold_sweep --scenario bs1 --metadata-path crates/forecast/checkpoints/anchors/tcn-bs1-…metadata.recalibrated.json --out-dir <tempdir>`

### `#133` — the "re-sync trigger" re-implemented its own subject, and importing the constant did not fix it
**Status**: FIXED 2026-09-29 (story 1-25 AC3.1). Anchor-impacting: **no** — pure move plus doc
corrections; no report body changes, `ANCHORS PASS (119 / 119)` before and after.

`crates/forecast/tests/sharpe_comparison_determinism.rs` is the test the design named the **re-sync
trigger** for the hourly annualisation constant. It re-declared that constant and re-implemented all
five metric functions — 90 lines — plus a copy of the report renderer, because the real ones lived
inside `bin/sharpe_comparison.rs`'s private `mod metrics` and a test cannot import a binary's module.
So it compared a copy against a copy: it would have stayed green if the binary's module were deleted,
and it could never trip on the drift it existed to catch.

The module (232 lines, 5 public functions, 36 call sites) now lives at
`crates/forecast/src/metrics.rs`; the bin and the test import the one definition. Pure move — the code
is byte-identical, dedented one level — and it brought 5 inline tests into the library suite with it
(83 → 88 with `--features candle`).

#### The part worth the entry: the obvious fix was not the fix

Importing the production constant was necessary and **not sufficient**. Probed it: swapping
`SQRT_HOURS_PER_YEAR` for √8760 — the value the docs claimed — left the test **green**. Its single
assertion renders a report twice and compares the renders, so a changed constant changes both sides
equally. A determinism tautology, the same shape the 1-21 review found in a different test.

**A gate that imports the right value can still be unable to fail.** What it needed was a PINNED
expectation, which is now `sqrt_hours_per_year_is_the_ratified_8575_constant_and_not_8760` — probed in
both directions: constant swapped → RED with the re-lock instruction in the message, restored → green.

#### The docs were the actual AC3.1 gap, and they were false in two crates

`backtest/src/stats/mod.rs:35` said *"Formula: … * sqrt(24*365)"* and
`forecast`'s metrics said *"√(24 · 365) ≈ 92.601295"*. √(24·365) = √8760 ≈ **93.594872**. The shipped
constant is √8574.9998 ≈ 92.601295 — so both docs named a formula ~2.1 % away from the code, and the
forecast one contradicted itself inside a single line. AC3.1 permits ratifying √8575 *with the doc
corrected to match*; the value was already ratified in a comment, so the docs were the whole gap.
Both corrected, and `stats/mod.rs`'s constant de-duplicated — it was declared identically inside two
functions, which is two places for one value to drift.

#### Scoped out deliberately

The test still carries its own copy of `render_report`, so `test_render_deterministic` continues to
test a copy of the renderer rather than the renderer. Extracting that is a larger move tied to the
bin's report shape, and it is not what AC3.1 names. Recorded here rather than done quietly.

**Still owed by a re-emission, not by this commit:** the renderer prints the literal string
`sqrt(24*365) = {:.6}` into hashed bodies at `sharpe_comparison.rs:1007/:1345/:1681`. That text is
false in the same way the docs were, and per `#110` a corrected engine behind an unchanged renderer
says the same words — so it cannot be cleared by a re-lock alone and rides the next re-emission of
those reports (story 1-29 covers three of them).

> **PAID 2026-10-01 — and it cost nothing, because the ride arrived.** Story 1-29's measurement found
> **all three** `sharpe-comparison` rows drifted (their sub-scenarios were re-emitted on 2026-09-26,
> after these rows were locked). All three bodies were therefore being re-emitted under § D6.b
> regardless, so correcting the label is free rather than a re-emission of its own. Six sites, three
> render families (`:776/:783` vol-target, `:1114/:1121` rebaseline, `:1450/:1457`
> regime-dispatcher), now reading
> `sqrt(8574.9998) = {:.6} (hourly -> annual; NOT sqrt(24*365) = 93.594872)`. The parenthetical stays
> in the body deliberately: the confusion is natural, the body is what gets quoted, and a reader who
> knows the hourly count is 8760 should be met with the answer rather than left to find the
> discrepancy.
>
> Had even one of the three been GREEN, this would have been a decision rather than a freebie —
> correcting its label would have forced a re-emission of a reproducing row. Worth stating because
> the cheap version of this choice was luck, not planning: nothing scheduled the label fix for the
> moment its carrier appeared, I happened to be holding both at once.

**2026-09-29 — `#128b` FIXED: the nine silent zeros are three paths, not two, and the third was never
written down.** `bin/threshold_sweep.rs::parse_gate_survivors` returned `[0usize; 9]` on failure and
its own doc called that *"graceful degradation"*. Those zeros then rendered into a **hashed body**
indistinguishable from nine measured zeros, with no log line anywhere.

Enumerating the failure paths while fixing it turned up one nobody had listed: besides (1) an
unreadable file and (2) a missing section heading, there is (3) a **partial parse** — fewer than nine
table rows left the tail silently zero-filled. The probe output is the defect stated in one line:

```
[69085, 60339, 51964, 44375, 0, 0, 0, 0, 0]
```

Four measured values and five fabricated ones, identical in the rendered table.

All three are now errors carrying their own diagnosis, and four unit tests pin them — including a
non-vacuity test that the happy path still parses, without which the three failure tests would pass
against a parser that could never succeed. **Probed**: relaxing the partial-parse guard turns the
third test RED with the array above; restored, 4 pass.

Why it mattered beyond tidiness: the input is a **hardcoded dated path into the anchored corpus**
(`evidence/v1/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md`),
and this project's re-emission pattern writes a NEW timestamped filename beside the old one. Those two
predecessor rows are themselves on story 1-29's list of scenarios awaiting a gate. The day they are
re-emitted, this read starts failing — and until today it would have started lying instead. The error
message says exactly that, so the next reader is not left to rediscover it.

Body-neutral by construction (only failure paths change) **and** measured: a full bs1 run after the
change re-emitted the body at `924a51bb…`, byte-identical to the digest it was re-pinned to hours
earlier. Process note, because this session is about exactly this: that sentence was written while
the run was still going and was confirmed afterwards. It happened to be right. Writing it before
the measurement landed was not, and the only reason it is not a defect in this record is that I
went back and checked rather than moving on.

### `#128c` — a run that exited 0 and wrote nothing still produced a verdict, computed from fabricated zeros
**Status**: half FIXED 2026-09-30 (story 1-29 AC5); the naming half rides the next re-emission.
Anchor-impacting: **no** for the fix; the rename is body-affecting and deferred deliberately.

`crates/forecast/src/bin/regime_verdict.rs` spawns the `backtest` binary, then looks for the report it
should have written. When it found none it substituted `String::new()` **unconditionally**, and every
downstream statistic then parsed as zero.

Two cases hide behind that one line, and only one of them is harmless:

- **The child FAILED.** Fine. V-REG-1 fires and its evidence string says *"Backtest did not complete
  successfully — EM convergence failure suspected"*, which names the real condition and hedges. The
  audit read this as a pure misdiagnosis; reading the code, it is more honest than that.
- **The child SUCCEEDED and wrote nothing.** Not fine. The verdict is then computed from fabricated
  zeros on a run that reported success, and nothing anywhere says so. That is this week's recurring
  shape: **a substitution made so the code can proceed, whose fact is then discarded.** It now bails
  with the reason, because a success that produced nothing is not a result any more than a refusal is.

#### What is NOT fixed here, and why it waits

The verdict's *name* still reads `V-REG-1 (Convergence failure)` / *"EM convergence failure"* while
what it actually tests is *"the backtest completed"* — the table row already labels itself honestly
(`| V-REG-1 | EM convergence (backtest completed successfully) |`), so the gap is the headline, which
is what gets quoted. Renaming it changes the hashed body of `regime-verdict-bs1-realdata`, which is one
of the ten rows story 1-29 will re-run and gate. It rides that re-emission rather than earning one of
its own.

Recorded rather than done quietly, because a rename that never happens is indistinguishable from a
rename nobody thought was needed.

---

### `#134` — the one line asserting the run was read-only names the only file the run wrote
**Status**: FOUND 2026-10-01 (story 1-29 AC7). Anchor-impacting: **yes** — the sentence is inside the
hashed bodies of `recalibrate-sigma-train-bs1` and `-bs2`, so fixing it is an ADR-0038 § D6.b
re-emission. **Not yet fixed**; the ruling and the plan are below.

`crates/forecast/src/bin/recalibrate_sigma_train.rs:457` renders, into the hashed body:

```rust
writeln!(&mut body, "- Read-only against `{overlay_path}` original safetensors.")
```

fed from `:686` with `&overlay_path.display().to_string()`. And `overlay_path` is built at `:628`:

```rust
let overlay_path = args.anchor_dir.join(&overlay_filename);   // {prefix}-{sha}.metadata.recalibrated.json
write_overlay(&original_metadata, sigma_train_recal, &overlay_path)?;   // :631 — the run WRITES it
```

So the body's sole read-only provenance claim names **the file the run created, eight lines earlier**,
and calls a `.metadata.recalibrated.json` *"original safetensors"*. Both anchored bodies carry it
verbatim:

```
- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d….metadata.recalibrated.json` original safetensors.
- Read-only against original `.metadata.json` (no mutation).
```

The second line is correct and needs no path. The first was meant to be its safetensors sibling and
was handed the write target instead.

The bin's own header doc states the contract correctly — *"NO mutation of … `.safetensors`"*, *"Exactly
two filesystem-write calls: 1. `std::fs::write(overlay_path, …)`"*. So the contract was right, the
code honours it, and only the **report** misstates it. That is the worst place for it to be wrong: the
report is the artefact a third party reads, and `verify_anchors.sh` has been defending the false
sentence since 2026-05-21.

#### Why this settles story 1-29 AC7, rather than being a separate finding

AC7 framed the two blocked rows as a choice between two defensible options:

- **(A)** accept a repo-mutating gate — run with `--anchor-dir` at its default (which rewrites the two
  committed overlay JSONs) and assert `git diff --exit-code crates/forecast/checkpoints/anchors/`;
- **(B)** move the path out of the hashed body, which is itself a body change and therefore a D6.b
  re-emission.

`#134` makes (A) untenable. That gate's procedure is *"run it, it rewrites file X, assert X came back
byte-identical"* — and the body it would be certifying says X is read-only. The gate would be defending
a sentence its own method contradicts. That is story 1-29 AC8's language about the threshold-sweep
lane, in a second lane: **converting an undisclosed behaviour into a defended invariant.**

**Ruled (b), recorded not escalated: fix the sentence, re-emit under D6.b, then gate.**

#### The fix also removes the blocker, which is the signal it is the right one

`#128e` blocks rows 5/6 because `--anchor-dir` — the *write* target — reaches the hashed body. The
*read* paths do not: `:506` is a hardcoded `PathBuf::from("crates/forecast/checkpoints/anchors")`,
independent of every flag. So a sentence that names the safetensors it actually read is stable under
`--anchor-dir <tempdir>`, and the gate sandboxes cleanly — one change closes a false claim **and** the
blocker, without the two being traded off against each other.

Print the relative constant, not a resolved path: `resolve_anchors_dir` (`tcn.rs:1489-1519`) falls back
to an absolute `CARGO_MANIFEST_DIR`-derived path when the CWD-relative probe misses, so a resolved
location would make the body machine-dependent — `#132`'s shape (a body that records *where the run
looked*) rather than a cure for it.

#### The shape, for the register

This is the week's recurring mechanism pointed at prose instead of at code: **a claim that cannot be
false.** Nothing re-reads that sentence against the filesystem, nothing cross-checks the path against
the writes the bin declares, and the body-SHA gate pins whatever it says. A sentence is exactly as
unfalsifiable as a gate with no assertion, and it is quoted more often.

Two siblings already on the books, same class, both deferred to the re-emission that will carry them:
the `sqrt(24*365) = 92.601295` label in the `sharpe_comparison` bodies (the value is √8574.9998, not
√8760), and `V-REG-1`'s *"EM convergence failure"* headline over a check that tests *"the backtest
completed"* (`#128c`). Three false strings in hashed bodies, found in one week, none of them caught by
anything — because nothing was looking.

#### And the test that should have caught it cannot — filed separately as `#135`

`recalibrate_sigma_train_readonly.rs::test_originals_untouched_by_run` is the guard that exists for
exactly this. It cannot fail, for three independent reasons, and it has **two siblings with the same
three reasons** — one per bin in this family. That is a cluster, not a footnote to `#134`: see `#135`.

#### The would-have-caught test (ADR-0038 § D6.b step 3)

The invariant, stated so it can only be satisfied by the fix: **the hashed body does not depend on
`--anchor-dir`.** Two runs into two different tempdir anchor-dirs must produce the same body-SHA.
Today they cannot — the directory is interpolated into the Notes line — so the test is RED against
current `HEAD` before the fix and GREEN after, which is the whole requirement. It is also exactly the
property that unblocks the two rows, so the re-emission and the gate are proved by one assertion
rather than two.

A cheap always-running companion sits beside it (the expensive one is ~16 min): a source walk
asserting `overlay_path` does not reach `render_report` at all. The `#133` lesson applies — the walk
must not be able to match its own search string.

---

### `#135` — the three read-only guard tests have never asserted anything, for three independent reasons each
**Status**: FOUND and FIXED 2026-10-01 while writing `#134`'s would-have-caught test.
Anchor-impacting: **no** — these are tests; no report body moves.

One "read-only guard" test exists per bin in the checkpoint-touching family:

| test | crate | the literal it reads |
|---|---|---|
| `recalibrate_sigma_train_readonly.rs:98` `test_originals_untouched_by_run` | `forecast` | `crates/forecast/checkpoints/anchors` |
| `threshold_sweep_readonly.rs:101` `test_originals_untouched_by_run` | `backtest` | `crates/forecast/checkpoints/anchors` |
| `forecast_distribution_bin_readonly.rs:88` | `forecast` | `crates/forecast/checkpoints`, `crates/forecast/replay-cache` |

All three are the same code. All three cannot fail, and each carries all three mechanisms:

**1. The "run" is `--help`.** Every one of them spawns the bin with `--help`, which clap handles and
exits before `main` reaches a filesystem call. The contract is asserted across an invocation that
cannot violate it. None of this is hidden — each doc comment says *"unchanged by a `--help`
invocation"*. The name was believed instead of the sentence directly under it.

**2. The paths do not resolve.** The literals are **workspace**-relative; `cargo` runs an integration
test from the **package** root. From `crates/forecast/` the first one resolves to
`crates/forecast/crates/forecast/checkpoints/anchors`; from `crates/backtest/` the second resolves to
`crates/backtest/crates/forecast/…`. Verified rather than reasoned: `os.path.isdir` on the literal
from the package root returns `False`. Every sentinel is missing on every run.

**3. A missing sentinel compares equal to itself.**

```rust
.map(|p| p.metadata().ok().and_then(|m| m.modified().ok()))   // → None when the path is wrong
```

`assert_eq!(None, None)` passes. Given mechanism 2 this is not a hypothetical about a deleted
checkpoint directory — it is what executes on every run.

Any one mechanism alone is sufficient. Together they mean these three tests have been green since they
were written, in both directions, and would stay green with the checkpoint directory deleted and the
bins rewritten to overwrite it.

#### It is a recurrence of `#119`, found by needing it rather than by looking

`crates/forecast/src/tcn.rs` carries a doc comment that describes this bug, by number, in the same
words — *"true of binaries launched from the repo root and **false under `cargo test`**, which uses
the PACKAGE root — so the path resolved to `crates/forecast/crates/forecast/checkpoints/anchors`"*.
That is `#119`, found 2026-09-26, fixed where its symptom was, and documented unusually well.

The three guards here had the identical literal and were never touched, because `#119`'s fix list was
*"`tcn.rs` and `patchtst.rs`"* — the two files where something visibly broke — rather than *"every
workspace-relative path literal in a crate whose tests run from the package root"*. Sixteen entries
later the same bug was still sitting in four more places, under a doc comment explaining it.

`#126` named this exact failure for a different list: **derive the member list from the criterion, not
from the symptoms.** It applies to a bug's fix list as much as to a gate's coverage list, and that is
the generalisation worth keeping from this pair.

#### Why this one is worth its own number

`#134` is a false sentence in a report. This is the instrument that was installed to detect exactly
that class of thing, reporting PASS. The register's recurring rule — *a gate that cannot fail is
indistinguishable from a gate that passes* — usually turns up one mechanism at a time. Here three
stacked in one function, written together, and the redundancy is what kept it invisible: fixing any
single one would have left the test still green, so anyone who checked one mechanism and moved on
would have come away reassured.

Note also what it took to find: not reading the tests, which I had done, but needing a REAL read-only
assertion for `#134` and discovering there was nowhere to put it because the existing one did not work.
The gap was visible only from the direction of someone trying to use it.

#### Fixed 2026-10-01, and PROVED non-vacuous

A rewritten test that passes is exactly what the old one did, so the pass was probed rather than
believed: a fifth, deliberately non-existent sentinel was appended to the list and the guard went RED
with *"sentinel … is MISSING or unreadable … This is UNMEASURED, not a pass"*. That is the proof the
whole pass rests on — under the new code the test **cannot** pass unless every sentinel was actually
read, which retires mechanisms 2 and 3 together.

The probe itself was checked for being a probe (the 1-27 pass had two that were no-ops reporting OK on
unmodified files): the file's SHA-256 before and after the mutation were compared, and the run only
counted because they differed.

#### The fix

- resolve from the **workspace** root (`CARGO_MANIFEST_DIR`'s grandparent), as the rest of the repo does;
- assert each sentinel **EXISTS** before reading it — the non-vacuity check (`#113` req 6) that would
  have surfaced mechanism 2 on the day it was written;
- compare **bytes**, not mtimes: stronger, free, and immune to a filesystem that does not update mtime;
- keep the `--help` case — it is a real if weak property — and put `help` in its NAME so it stops
  standing in for a run;
- put the real-run assertion where a real run already happens: `#134`'s anchor-dir-invariance gate,
  which runs the bin twice anyway and can assert the committed checkpoints are byte-identical after.

**Process note, 2026-10-01 — the probe's restore destroyed the thing it was probing.** The probe
script restored with `git checkout -- <file>`. That file carried the **uncommitted** `#135` fix, so
the restore reverted to HEAD and silently threw the patch away; it was caught only because the
before/after SHA-256 comparison — the no-op guard, there for an unrelated reason — printed two
different hashes at the end. A snapshot (`cp` to a temp file, `cp` back) is the correct restore for a
working tree with uncommitted work, and the probe harness now uses one. Recorded because the guard
that caught it was looking the other way: without it the probe would have reported a clean red and
left the fix gone.

---

### `#136` — the determinism test renders its OWN copy of the report twice, and the copy has drifted
**Status**: FOUND and FIXED 2026-10-01 while checking what else consumed the body line `#134`
changed. Anchor-impacting: **no**. The `#133` sibling that was left behind, now with a measurement
rather than a suspicion.

`crates/forecast/tests/sharpe_comparison_determinism.rs:52`:

```rust
fn render_report(results: &[RerunResult; 4], _ctx: &ReportContext) -> String {
```

That is a **test-local re-implementation** of the renderer in `sharpe_comparison.rs`. The test that
uses it, `test_render_deterministic` (`:238`), renders it twice and compares the two renders.

Two things follow, and the second is the one that was not known before today.

**It cannot fail in the way its name suggests.** It proves a function the product never calls is
deterministic. Any change to the real renderer — including one that made it non-deterministic — leaves
it green. The file already says this about its *other* half, in the docstring added on 2026-09-29:
*"it renders a report twice and compares the two renders. Changing the constant changes both sides
equally, so it stays green — a determinism tautology."* That sentence was written about the
annualisation constant and is just as true of the whole render.

**And the copy HAS drifted — measured, not feared.** Its notes line reads:

```
"- Read-only against the four -realdata reports listed in frontmatter."     (test copy, :163)
"- Read-only against the five -realdata reports listed in frontmatter."     (the bin, :495)
```

Four versus five, and the signature is `&[RerunResult; 4]` against a bin arm that re-runs five
scenarios. So this is no longer "two definitions that could diverge": they *have*, and nothing noticed,
because the only thing comparing them is the copy against itself.

#### Why it is being retired rather than re-synced

`#133` fixed this file's other half the right way — the metrics were extracted to
`forecast::metrics` so bin and test share one definition. The renderer was left, and the usual reason
to keep a copy is that it is the only coverage there is. As of today it is not: the three
`sharpe-comparison-*` rows have real reproduction gates in
`crates/forecast/tests/anchored_report_reproduction.rs`, which re-run the **actual** binary and compare
against the **anchored** body. That is strictly stronger than any self-comparison, and it is the
condition under which deleting a test is safe — something real replaces it.

`sqrt_hours_per_year_is_the_ratified_8575_constant_and_not_8760` stays: it compares against a pinned
number, which is exactly why it was written and why it can fail.

**Deletion was HELD until the replacement was measured, and then released the same day.** The
argument above turns on the three `sharpe-comparison-*` reproduction gates being real coverage, and
when it was written they had not reported. Removing the old test on the strength of a replacement
whose state is unknown is the same move as re-pinning an anchor to current output: it assumes the
answer.

They reported at 14:06. **All three RED**, each with its old and new digest and every witness passing
first — a real drift that the self-comparison was structurally incapable of seeing, since the copy was
only ever compared against itself. That is the condition met, and in the direction that makes the case
strongest: the replacement did not merely exist, it *caught something*.

Retired accordingly. `sharpe_comparison_determinism.rs` keeps the one test that can fail — the
ratified √8574.9998 constant, pinned against a number — and loses 8 kB of mirrored types, copied
renderer and the tautology. Per ADR-0082 the evidence moved, so the trace moved with it:
`trace.toml`'s T-D-10 row now lists `anchored_report_reproduction.rs` as the renderer evidence and
describes what is left behind honestly.


---

### `#137` — a re-emission flipped a verdict to ALPHA-UNLOCKED, and it is the row with the synthetic baseline
**Status**: ESCALATED to the operator 2026-10-01, unlanded. Anchor-impacting: **yes** — the body
exists and is held out of `evidence/` pending the ruling. AD-19 / public-claim territory.

Story 1-29's AC6 measurement found all three `sharpe-comparison` rows drifted, because their
sub-scenario bodies were re-emitted under § D6.b on 2026-09-26 (bug-log `#94`'s sizer fix, among
others) after these rows were locked in May. Re-emitting them is routine. **One verdict moved.**

| row | baseline | overlay/dispatcher | Δ | verdict |
|---|---|---|---|---|
| `vol-target-bs1-realdata` **(was)** | −0.026770 | −0.018621 | +0.008149 | `T-VOL-NO-ALPHA` |
| `vol-target-bs1-realdata` **(now)** | **−0.667765** | **−0.407398** | **+0.260366** | **`T-VOL-ALPHA-UNLOCKED`** |
| `vol-target-bs1-realbaseline` (was) | +0.003098 | −0.018621 | −0.021719 | `T-VOL-NO-ALPHA` |
| `vol-target-bs1-realbaseline` (now) | −0.328302 | −0.407398 | −0.079096 | `T-VOL-NO-ALPHA` |
| `regime-dispatcher-bs1` (was) | +0.003098 | −0.291015 | −0.294113 | `T-REG-NO-ALPHA` |
| `regime-dispatcher-bs1` (now) | −0.328302 | −0.290 … | +0.037287 | `T-REG-NO-ALPHA` |

#### Three readings the operator needs before ruling, and they point the same way

**1. Both Sharpes are NEGATIVE.** −0.667765 and −0.407398. The overlay does not make money; it
**loses less**. `T-VOL-ALPHA-UNLOCKED` is literally correct under its own rule — ADR-0038 § D1.c says
`net_delta >= 0.10 → T-VOL-ALPHA-UNLOCKED`, and the rule says nothing about the sign of either side —
and it invites a reading the numbers do not support. Same species as everything else filed this week:
a label that says more than its measurement.

**2. The flipped row is the one with the SYNTHETIC baseline, and the overlay is identical in both.**
The same overlay scores **−0.407398 in both rows**. The entire difference is the baseline:
`top10-2023-1h-momentum` (synthetic, forced to `Linear { bps: 8 }` under Q-D1=(a)) at −0.667765,
versus `top10-2023-fy-momentum-realdata` (real Binance data) at −0.328302. The synthetic baseline is
0.34 Sharpe worse, and that gap is the whole of the +0.26 "alpha".

**3. The project already built the better comparison, and it says NO-ALPHA.** The
`-realbaseline` row exists *because* the synthetic baseline was judged the wrong reference — that is
what "rebaseline" means and why it was locked a week later. Same overlay, real baseline, Δ = −0.079:
the overlay is **worse** than the baseline it should be measured against.

So the flip is an artifact of the weaker of two comparisons the repo deliberately keeps side by side,
and the stronger one is unchanged. **The era-qualified thesis is not contradicted by the better
measurement.** But a body that prints `T-VOL-ALPHA-UNLOCKED` is a change in what the corpus claims,
whatever the surrounding prose says, and the prose is not what gets quoted.

#### Why this is a STOP and not a decision I take

Landing it changes what the project publicly claims. That is one of the four standing stop conditions,
and AD-19 forbids shipping a moved verdict without an explicit human override. The other two rows'
verdicts did not move and are landed.

#### Not fixed, and NOT to be fixed by softening the number

The resolution is **not** to re-pin, re-scale, or re-thershold anything — that is `#77` with a
different mask. The options are about what the body SAYS, not what it computes, and the FROZEN gate
(AD-1) is untouched either way: `classify_verdict` / `verdict_bands` / `compute_robustness_flag` /
`rank_candidates` are not in this path at all. This is ADR-0038 § D1.c's T-classifier, a report-level
label.
