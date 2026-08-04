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
**Status**: OPEN — implement-or-drop decision rides story 1-25's AC3 ratify-or-fix list (with the axis's narrative corrected at regeneration). Disclosure entry, 2026-08-03.
**Discovery**: story 1-16 code-review Blind Hunter H1; orchestrator-accepted on the reviewer's caller-graph evidence.
**Mechanism**: `drift_rebalance_threshold` reaches exactly three places — the config hash, the report grid-definition table, and `MomentumStrategy.drift_threshold` marked `#[allow(dead_code)]` (written, never read). No drift/hold-band logic exists in `run_path`/`PaperEngine`; the only real implementation (`risk::size_portfolio_target`) has zero production callers; the equal-weight open/close-on-membership signal scheme cannot express a hold band. 0.10 vs 0.30 vs 0.50 changes nothing.
**Impact**: the anchored θ-surface narratives (#86 momentum, #87 MR) attribute cell results to "wide hold-band / low-turnover engineering" — a confounded interpretation (lookback+k carry everything). Verdicts stand (all-FRAGILE is direction-preserving under an inert axis); the INTERPRETATION and the grid's third-axis scientific claim do not. No test could go red on this: no cell pair is drift-only distinct.
**Moral** (the #65 lineage, again): a parameter that is hashed, printed, and swept is not a parameter that is EXECUTED — the same class as the v3-vol no-op overlay (#65), one layer up. Sweep design must include a per-axis "this axis moved the output" probe (a drift-only cell pair would have caught this on day 1).

### `#69` — `portfolio_exposure_cap` is INERT engine-wide; D-TSM.2's ratified safety premise is false; the anchored TS surfaces ran ~2× the documented gross exposure, alphabetically rationed
**Status**: OPEN — enforce-or-delete + corrected exposure description + explicit thesis re-affirmation ride story 1-25 AC3/AC4. Disclosure entry, 2026-08-04.
**Discovery**: story 1-17 code-review Blind Hunter H-1 (the #68-mandated caller-graph probe, aimed at the risk-limit layer).
**Mechanism**: `Order::new` validates ONLY `per_symbol_exposure_cap` (`crates/core/src/order.rs:123-170`); `portfolio_exposure_cap`'s sole implementation (`crates/risk/src/portfolio.rs:189`) has zero production callers; `run_path` sets `Some(dec!(0.50))` decoratively with empty per-order Position snapshots. Invisible in every top-K family (K≤5 × 10% fixed-fraction ≤ 50% — the cap could never bind).
**Impact**: story 1-17's LOCKED design certified fixed-fraction sizing as safe BECAUSE "the 0.50 portfolio cap throttles" high-cardinality bars — false. TS long/flat emits up to 10 Buys → ~90-100% gross in high-breadth regimes (anchored tim 0.78-0.87) vs the hashed `held_constant | exposure_cap=0.50` row in anchors #90/#91; the only real limiter is the cash pre-flight, which rations ALPHABETICALLY (BTreeMap emission order), starving alphabetically-late symbols — violating the design's own per-asset-independence criterion. The FAMILY-UNIFORM-FRAGILE verdict likely survives (p5-Sharpe ≈ exposure-scale-invariant; margins ≥~1.06 Sharpe uniform), but prob_loss/p95-maxdd are exposure-sensitive banded signals and the anchored body misdescribes the book — the 1-25 regeneration must correct the description and re-affirm the closure explicitly.
**Moral** (the #68 lineage, risk-limit edition): a limit that is set, printed, and ratified into a design's safety argument is not a limit that is ENFORCED. Every declared risk control needs a binding test (construct a scenario where the limit must bind; assert it does).

## Changelog

- 2026-05-25 (orchestrator): file created. Backfilled #54–#63 from `git log` + inline `Bug #N` comments.
- 2026-05-25 (orchestrator): #64 added — progress bar short-run starvation fix.
- 2026-05-26 (orchestrator): #65 added — vol_killswitch_overlay no-op discovered by Wave 1 overlay-e2e test; 2 tests `#[ignore]`-gated pending source fix.
- 2026-05-26 (analyst): #65 updated — analyst brief authored at [`spec/vol-killswitch-overlay-noop-fix v0.1.0`](../archive/pre-bmad-spec/v1/vol-killswitch-overlay-noop-fix/feature.md). P0 safety; trace row `REQ-VOL-KILLSWITCH-NOOP-FIX-001` at `proposed`; sibling of shipped `v3-volatility-forecaster-noop-fix v0.1.0` 2026-05-22. Status flipped `open` → `open (analyst brief authored)`.
- 2026-05-26 (developer): #65 FIXED — Q4=(p3) "Both" fix shipped. A.1: lookback_minutes 60→5 + flat warmup prevents GARCH early-kill. A.2: overlay filter broadened to basket-wide Hold. A.3: #[ignore] removed; 4/4 tests green. Hygiene gate 2/2 pass.
- 2026-08-04 (orchestrator): #69 added (OPEN) — portfolio_exposure_cap inert engine-wide (the #68 lineage at the risk-limit layer); D-TSM.2 premise false; TS surfaces ran ~2× documented gross; enforce-or-delete + thesis re-affirmation ride 1-25. #67 blast radius extended to anchors #90/#91.
- 2026-08-03 (orchestrator): #68 added (OPEN) — the θ-grids' drift/hold-band swept axis is inert (the #65 class, one layer up); implement-or-drop rides 1-25; #67 blast radius extended to anchor #87.
- 2026-07-31 (orchestrator): #67 added (OPEN) — cross-symbol fill mispricing in the research-harness lanes; anchored C2/C3 evidence is execution-artifact noise; advisor gate proven unaffected; fix+re-lock = story 1-25 (program with 1-24).
- 2026-07-27 (orchestrator): #66 added+FIXED — ui real-data guard tests vacuous since day 1 (cwd-relative corpus root, any-Err→skip); revival caught 3 latent production bugs (CSV-name test bug, scenario-name collision/shadowing, unindented-frontmatter Compare skip). Story 1-10 code-review pass; all gates re-verified (anchors 119/119, spec-lint 0, clippy 0, AC5 4369-point round-trip).
