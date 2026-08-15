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

**⚠ SEQUENCING — do not enable the feature yet.** The one-line change that would make the arm capable is `crates/ui/Cargo.toml`: `backtest = { path = "../backtest" }` → `backtest = { path = "../backtest", features = ["yahoo"] }`. **It must not be flipped before the emission-cadence defect below is fixed**, because it would produce a *working-but-wrong* arm rather than an inert one — the union loop emits up to 3 records per trading day with legs of mixed vintage, so the pre-registered "3-AND rule at the daily close" is evaluated on aligned inputs roughly one time in three, and a day on which two legs flip yields two round-trips where the rule intends one. The recorded "6 regime flips" is itself a cadence-inflated count. An inert arm that is honestly absent is a better state than a live arm computing the wrong rule. (Note also that even with the feature on, the default NOW-anchored lookbacks would still yield absence — the corpus ends 2026-06 — but now **loudly**, with the gap named, which is what the honesty fix buys.)

**Cost clause (pre-registration departure) — documented, economics untouched, operator's call.** Two options, both anchor-neutral: **(A)** charge the pre-registered taker fee in place (~10 lines; the recorded −0.39% becomes roughly −0.63%, Sharpe slightly more negative, no crown effect at any of these values) — this closes the pre-registration gap but leaves the asymmetry; **(B)** route the arm through `PaperEngine` so it also pays slippage and lot-rounding — **only (B) makes its friction comparable to the 18 siblings it is ranked against**, which is what the finding is actually about, and it is the same fix shape #80 wants for short legs.

**Moral**: a `#![cfg(feature = "…")]` on a whole module makes its absence *silent* — the crate compiles, the caller takes the `cfg(not(...))` branch, and the feature simply is not there. Combined with per-crate feature namespacing, a workspace can enable `x/yahoo` everywhere it is visible and still leave `y/yahoo` off. **Never let a product capability depend on a feature flag no test asserts is on.** For every `cfg(feature)`-gated capability, ask: which build actually ships it, and what fails if it does not?

### `#82` — The advisor's entire SHORT SLATE never shorts on real data. Five arms are ranked as long/short; four are long-only in practice, one never trades at all, and the flagship ratchets to ~11-16× leverage because the side-blind cap refuses its exits.
**Status**: OPEN (found 2026-08-15 while fixing #80; the census that exposed it is now printed on every run so it cannot hide again). Anchor-impacting: **no** (advisor/bake-off lane, `write_report=false`). This is **#71's consequence on the product surface** — the research-lane defect was known; what is new is what it does to the ranked field the operator reads.

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

**What it does to the product.** The advisor tells the operator it evaluated a long/short slate. It evaluated four long-only arms and one no-op, and one of the four is a leveraged long that blows through its own risk limit and ends underwater. The *rankings* are not wrong — those arms genuinely performed as measured — but their **labels are**, and a slate presented as testing the short side tested nothing of the kind. As with #78 and #81, the harm is that the field over-represents what was actually explored.

**Why it stayed invisible**: nothing counted short legs. The KPI columns show fills and trades, not sides; the exposure-cap rejection is silent (bug-log #71's own moral); and no test asserted that a short-enabled arm ever took a short. The `[SHORT-CENSUS]` line added during the #80 fix is the fix for *that* — it prints `short_legs=0` with an explicit warning marker on every run.

**Fix direction** (none taken — this needs a decision, not a patch): the cap must stop refusing position-*reducing* orders (that is #71's fix, and it is the load-bearing one here); the alternating arms need either a signal shape that can reach flat or honest re-labelling as long-only; and `v0.always_short` needs to either trade or be dropped from the field per #81's drop-to-ABSENCE precedent. **Do not simply raise the cap** — that would let the ratchet run further, not fix it.

**Moral**: a capability in a strategy's *name* is a claim, and nothing was checking it. When an arm is labelled long/short, assert that it takes a short — on real data, not a fixture built to make it. The census that found this is four lines; the absence of those four lines let five arms misrepresent themselves for the product's entire life.

## Changelog

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
