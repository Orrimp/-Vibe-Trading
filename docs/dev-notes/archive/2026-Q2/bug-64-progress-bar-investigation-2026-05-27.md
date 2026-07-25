---
slug: bug-64-progress-bar-investigation-2026-05-27
status: living
owner: orchestrator
updated: 2026-05-27
---

# Bug #64 follow-up — Yahoo Lab progress bar still appears stuck

> **2026-05-27 fresh investigation.** Operator reported (paraphrased):
> "The graph showed everything in synthetic data set but for yahoo it
> was still stuck." Bug #64 was marked **fixed** at commit `7acb94b`
> (2026-05-25). This note documents the post-fix code-path audit,
> confirms the fix is still in place, and isolates the most likely
> residual UX issue plus an operator repro recipe so the next round
> can verify on demand.
>
> **Scope**: read-only investigation. No code touched. Files audited:
> `crates/ui/src/lab/progress.rs`, `crates/ui/src/lab/runner.rs`,
> `crates/ui/src/bin/cockpit_live.rs`, `crates/ui/src/state.rs`,
> `crates/ui/src/screens/lab.rs`, `crates/ui/src/widgets/progress_bar.rs`,
> `crates/backtest/src/scenarios/{sma_composed_run,momentum,pairs,tcn_overlay}.rs`,
> `crates/backtest/src/progress.rs`, `crates/data/src/yahoo.rs`.

## Root-cause hypothesis: **D — UX artifact, not a code regression**

The Bug #64 anchor fix (force-emit at the final bar in all 4 scenarios + Yahoo
preload sentinel in `runner.rs`) is **still present in the current tree**.
What the operator is now seeing is one or both of two distinct, surviving UX
artifacts that the #64 ship did not eliminate — and that the original `#64`
diagnosis on the 2026-05-25 30-bar daily run did not see, because the
day-derived interval logic was different then:

1. **(D.1) The Yahoo preload sentinel never animates.** During cold-cache
   Yahoo fetches (30–60 s on the first run for a ticker; ≤ 5 s on cache hits)
   the operator sees the bar parked at exactly 0 % with the static label
   `"0 / 1 bars · 0.0s"`. The `elapsed_ms` field is hard-coded to 0 at the
   sentinel emit site (`runner.rs:617-621`) and there is no second sentinel
   on the way to the engine. To a human watching for animation, this is
   indistinguishable from "stuck".

2. **(D.2) Post-preload engine completes in a single repaint.** Once
   `preload_yahoo_bars` returns, the engine runs in tens of milliseconds for
   typical Yahoo bar counts. The two emits (`bar 0` ≈ 0 % then `bar N-1` ≈
   97 %) usually compress into a single iced repaint frame, then
   `LabRunCompleted` immediately clears `lab_state.run_progress = None`
   (`state.rs:2167`) → the bar disappears entirely. Synthetic feels smoother
   because (a) there is no preload sentinel pause, and (b) for hourly
   synthetic data the engine emits ~10 events spread across more wall-clock
   time owing to larger `bar_count` after the deterministic-bar build.

A genuine `(A)/(B)/(C)` regression was ruled out — see "Verification" below.

## Code-path trace — `LabRunRequested` → progress bar pixel

| Hop | File:line | What happens |
|-----|-----------|--------------|
| 1 | `crates/ui/src/state.rs:1534, 2156-2163` | `Message::LabRunRequested` enum + pure-state arm: flips `lab_run_inflight = true`, clears `run_progress`, clears `last_run_error`. |
| 2 | `crates/ui/src/bin/cockpit_live.rs:1503-1514` | Binary-side wrapper: builds `(handle, cancel_recv)`, builds `(progress_tx, progress_rx)`, bumps `lab_progress_recipe_salt`, stores `progress_rx` in `Arc<Mutex<Option<_>>>`. |
| 3 | `crates/ui/src/bin/cockpit_live.rs:1533-1539` | Calls `ui::lab::runner::spawn_lab_run(...)` returning an `iced::Task`. |
| 4 | `crates/ui/src/lab/runner.rs:588-660` | Spawned task: if `data_source == YahooCache`, emits the **sentinel** `Progress { current_bar: 0, total_bars: 1, elapsed_ms: 0 }` (`runner.rs:617-621`) and then awaits `preload_yahoo_bars(...)` (cold cache → 30–60 s; warm → ≤ 5 s). |
| 5 | `crates/ui/src/lab/runner.rs:661-700` | Once preload returns, spawns `backtest::engine::run_scenario(scenario_cfg, cancel, progress_tx)` on the tokio rt. |
| 6 | `crates/backtest/src/scenarios/sma_composed_run.rs:416-441` (SMA) **or** `momentum.rs:312-329` / `pairs.rs:179-196` / `tcn_overlay.rs:183-200` (cross-sectional) | Bar-loop emits `Progress` per poll gate. **Bug #64 force-emit at final bar is present in all four scenarios.** SMA: `bar_idx == bar_count.saturating_sub(1) \|\| (bar_idx < 128 && bar_idx & 0x1F == 0) \|\| bar_idx & 0x7F == 0`. Cross-sectional: `bar_idx.trailing_zeros() >= 7 \|\| bar_idx == total_bars.saturating_sub(1)`. |
| 7 | `crates/ui/src/lab/progress.rs:96-112` (`stream_impl`) | `LabProgressRecipe::stream()` enters tokio runtime, takes the `Receiver`, then drains `recv().await` → yields `Message::LabRunProgress(progress)` per event. On channel close, yields `Message::LabRunProgressDone`. |
| 8 | `crates/ui/src/state.rs:2192-2198` | `Message::LabRunProgress(p)` arm sets `model.lab_state.run_progress = Some(progress)`. |
| 9 | `crates/ui/src/screens/lab.rs:417-437` | View reads `lab_state.run_progress`; derives `progress_pct = current_bar / total_bars` and label `"{cur} / {tot} bars · {elapsed}s"`; passes to `progress_bar::view`. |
| 10 | `crates/ui/src/widgets/progress_bar.rs:54-94` | `progress_bar::view` renders fill; `None` → indeterminate static 30 % fill (`INDETERMINATE_FILL = 0.30`, line 40). |
| 11 | `crates/ui/src/state.rs:2164-2174` (`LabRunCompleted`) | On completion: `lab_run_inflight = false`, **`run_progress = None`** (line 2167) → bar element drops out of the row entirely. |

Channel implementation: `crates/backtest/src/progress.rs:64-67` — `tokio::sync::mpsc::channel(8)`, lossy, non-blocking `try_send`.

## Why synthetic feels smooth and Yahoo feels stuck (the D hypothesis in detail)

| Phase | Synthetic Last30d (BTCUSDT hourly) | Yahoo Last30d (BTC-USD hourly) cold cache | Yahoo Last30d (BTC-USD hourly) warm cache |
|-------|------------------------------------|-------------------------------------------|-------------------------------------------|
| `LabRunRequested` → `Task::perform` body entered | < 50 ms | < 50 ms | < 50 ms |
| Yahoo preload sentinel emit | (skipped — synthetic path) | `Progress { 0, 1, 0 }` rendered as "0 / 1 bars · 0.0s" | "0 / 1 bars · 0.0s" |
| Preload await | n/a — synthetic bars generated inline (~ ms) | **30–60 s of static "0 / 1 bars · 0.0s"** ← *this is what the operator perceives as stuck* | ≤ 5 s of static "0 / 1 bars · 0.0s" |
| Engine bar-loop start | total_bars = 720; ~ 10 emits | total_bars = ~720 (BTC) or ~150 (stock); ~ 2–10 emits | same |
| Engine duration | tens of ms wall-clock | tens of ms wall-clock | tens of ms wall-clock |
| Final-bar emit (Bug #64 fix) | bar 719, ~ 99.9 % | bar N-1, ~ 99 % | bar N-1, ~ 99 % |
| `LabRunCompleted` clears `run_progress` | bar disappears | bar disappears | bar disappears |

Synthetic gets ~ 10 visible animation frames *across* the engine because the
720-bar synthetic SMA loop interleaves enough work between emits to span more
than one iced repaint. Yahoo gets ~ 2 frames because (a) the engine has fewer
bars on stocks, or (b) the engine is so cheap relative to iced's draw cadence
that emits coalesce. Either way, the **dominant visible state in the Yahoo
flow is the preload sentinel sitting static** — that is the "stuck" feeling.

## Verification — the #64 fix is still in place

- `crates/backtest/src/scenarios/sma_composed_run.rs:426` —
  `bar_idx == bar_count.saturating_sub(1) || ...` ✅
- `crates/backtest/src/scenarios/momentum.rs:320` —
  `bar_idx.trailing_zeros() >= 7 || bar_idx == total_bars.saturating_sub(1)` ✅
- `crates/backtest/src/scenarios/pairs.rs:187` — same shape ✅
- `crates/backtest/src/scenarios/tcn_overlay.rs:183` — same shape ✅
- `crates/ui/src/lab/runner.rs:617-621` — preload sentinel emit ✅

The v5-latency-slippage-sim commit (`a5f8647`) and the
cockpit-training-pressed-wiring commit (`910fa0f`) did **not** modify any
file on the lab progress path; only additive plumbing in
`runner.rs:466-467` (new `latency_slippage_sim` field on `ScenarioConfig`)
and a parallel `TrainingLogRecipe` in `crates/ui/src/lab/training_log.rs`
that is symbol-for-symbol the same shape as `LabProgressRecipe` (no
cross-contamination).

Iced subscription wiring is correct: `cockpit_live.rs:1587-1592` constructs
`LabProgressRecipe` per `lab_progress_rx.is_some()` with a salt-bumped hash
that forces iced to call `stream()` fresh on every `LabRunRequested`. The
2026-05-25 `tracing::warn!` probes already proved this end-to-end (see
`spec/bug-log.md#64`, "Probes used during diagnosis").

## Proposed fixes (NOT applied — operator approval required)

Three small, scoped options, in increasing intrusiveness. All within
`crates/ui/src/lab/{progress,runner}.rs` per scope guard (v5 dev holds the
backtest lock). All under 30 LoC and one file each.

### Option (D.1.1) — Animate the preload sentinel via a periodic ticker

While the Yahoo preload future is awaiting, spawn a sibling `tokio::time::interval`
loop that emits a sentinel-shape `Progress { current_bar: 0, total_bars: 1, elapsed_ms: <real> }`
every 250 ms with the wall-clock elapsed since preload start. Tear it down via
`tokio::select!` against the preload future. This makes the "0 / 1 bars · X.Xs"
label *tick visibly* during the wait — the operator sees movement.

Touched: `crates/ui/src/lab/runner.rs:599-660` only. Estimated diff: ~ 20–25
LoC. Risk: low — sentinel is already a wire-protocol convention; widget already
clamps `total_bars > 0` filter so cosmetics are unchanged. Anchor-safe by
construction (progress events are channel-only, never written to reports).

### Option (D.1.2) — Distinguish "preloading" from "engine running" in the label

Add a new `Message::LabPreloadStatus(elapsed_ms)` arm that updates a separate
`lab_state.preload_status: Option<u64>` field. The view renders this as
"Loading Yahoo data… 12.3s" when `run_progress == None && preload_status.is_some()`,
replacing the indeterminate-30 %-fill sentinel rendering altogether for the
Yahoo path.

Touched: `state.rs` (new field + new Message arm), `screens/lab.rs` (view
branch), `lab/runner.rs` (replace the sentinel emit with a periodic
`Message::LabPreloadStatus` poke via a separate `iced::Task`). Estimated diff:
~ 40–50 LoC across 3 files → **out of scope per the 30-LoC + 2-file limit**.
Document only.

### Option (D.2.1) — Latch the bar at "complete" for ~1 s post-LabRunCompleted

Replace the immediate `run_progress = None` in `state.rs:2167` with a
"linger" — keep the last `Progress` in state, but flip an `is_inflight` flag
to false so the view can decide whether to render. Then a short `iced::time::every(1s)`
clear-tick removes the bar after ~1 s. The operator sees a full sweep to 100 %
plus a beat of "done" before the bar vanishes.

Touched: `state.rs` + binary-side timer. Estimated diff: ~ 25 LoC across 2
files. Risk: low. **However**, this changes pure-state semantics and would
touch the existing `LabRunCompleted` contract — needs analyst sign-off before
applying.

**Recommendation**: Operator runs the repro recipe below first. If the
preload-phase pause is the dominant complaint → ship Option (D.1.1). If the
post-engine flash is the dominant complaint → ship Option (D.2.1). If both,
do (D.1.1) first (smaller blast radius), measure, then revisit (D.2.1).

## Operator repro recipe — confirm "stuck" reliably on demand

Following the 6-section format per AGENT.md § Communication contract (commit
`fe48fd7`). Recipe assumes a fresh state with the Yahoo cache cleared so we
can reliably trigger the cold-cache 30–60 s window — the most likely root of
the "still stuck" feeling.

### Command

```bash
# Tab 1 — clear the Yahoo cache for a fresh-ticker test, then launch the
# live cockpit with full lab/yahoo + lab.progress tracing.
rm -rf /tmp/yahoo-cache-bug64-snapshot
mv data/yahoo /tmp/yahoo-cache-bug64-snapshot   # safe stash, restore after
RUST_LOG=lab.yahoo=info,lab.progress.recipe=warn,backtest=info \
  cargo run --release --bin cockpit_live --features "live yahoo" \
  2>&1 | tee /tmp/bug64-repro.log
```

```bash
# Tab 2 — live probe of progress emission cadence.
watch -n 1 'grep -E "lab\.(yahoo|progress|run)|Progress \{|cache miss|fetch_and_cache" /tmp/bug64-repro.log | tail -40'
```

### Steps

1. **Open the cockpit** → switch to the **Lab** tab (left rail).
2. **Pick a fresh ticker** (anything other than BTCUSDT — try **`ETHUSDT`** so
   the cache is guaranteed empty): click the pair selector → ETHUSDT.
3. **Select a strategy**: click `sma_crossover` chip.
4. **Set date range**: click the **Last30d** preset chip.
5. **Switch the data source toggle to `YahooCache`** (top-bar toggle next
   to the cadence badge).
6. **Press Run**.
7. **Watch the progress bar widget next to the Run button** for the next
   60 s. Note (a) whether the label `"0 / 1 bars · 0.0s"` updates, (b) when
   the engine starts (look for `running backtest loop (... bars)` in tab 2),
   (c) the final bar percentage before the widget disappears.

### Expected timing

| Phase | Wall-clock | Visible state |
|-------|-----------|---------------|
| Click Run → first iced repaint | < 100 ms | Bar appears at 0 % with label `"0 / 1 bars · 0.0s"` |
| Yahoo cold-cache fetch | **30–60 s** | Bar parked at 0 %, label static at `"0 / 1 bars · 0.0s"` (THIS is the operator-reported "stuck") |
| Engine bar-loop runs | 10–100 ms | One or two repaint frames showing ~ 0 % → ~ 99 % |
| `LabRunCompleted` | next frame | Bar disappears entirely; chart populates with results |

### Expected result

- **CONFIRMS hypothesis D.1** if the bar visibly sits at 0 % with label
  `"0 / 1 bars · 0.0s"` for ≥ 5 s with no visible label tick. This is the
  cold-cache preload starvation.
- **CONFIRMS hypothesis D.2** if after preload completes, the operator sees
  no animation — just an instant disappearance / one frame of "high
  percentage" before the bar vanishes.
- **REFUTES** both if the bar updates smoothly during preload AND animates
  through several intermediate percentages during the engine run.

### Failure modes

- **Auto-fetch fails** (`Yahoo auto-fetch failed for ETH-USD: ...`) → the
  Run button shows a `⚠ ...` error message; the progress bar disappears and
  `last_run_error` is populated. This is Bug #54 / #63 territory, not #64.
  Restart from step 2 with a different ticker (try `MSFT` or `AAPL`).
- **Cache already warm** (no `cache miss` log line in tab 2) → the preload
  pause shrinks to < 5 s; the "stuck" feeling is much milder. Repeat the
  recipe with `rm -rf data/yahoo/ETH-USD/` between runs to keep cold-cache
  conditions.
- **Network down / Yahoo rate-limit** (`fetch timed out` in tab 2) → the
  60 s per-attempt timeout from Bug #63 kicks in, then retries with backoff.
  The bar will be static at 0 % for **up to 5 × 60 s = 5 min** before
  surfacing an error. This is the worst-case "stuck" pathology and likely a
  contributor to operator perception.
- **Cockpit panics on first frame** → run the `cockpit-smoke` skill first to
  confirm fixture-mode boot works before live-binary repro.

### Cleanup

```bash
# Stop the cockpit (Ctrl-C in tab 1).
# Stop the watch (Ctrl-C in tab 2).
# Restore the original Yahoo cache.
rm -rf data/yahoo
mv /tmp/yahoo-cache-bug64-snapshot data/yahoo
# Archive the probe log if useful.
mv /tmp/bug64-repro.log docs/dev-notes/bug64-repro-$(date +%Y-%m-%d).log
```

## Next steps (for orchestrator triage)

1. **Operator runs the repro recipe.** Capture which of D.1 / D.2 dominates.
2. **If D.1 dominates** → green-light Option (D.1.1) as a P2 follow-up bug
   under the `lab-end-to-end-v2` follow-up area. ~ 25 LoC in one file.
3. **If D.2 dominates** → green-light Option (D.2.1). ~ 25 LoC in two files;
   needs analyst sign-off on the `LabRunCompleted` pure-state contract change.
4. **If both** → ship D.1.1 first, observe, then revisit.
5. **If neither** (hypothesis refuted) → re-open the investigation with the
   probe log in hand; consider re-instrumenting `LabProgressRecipe::stream()`
   per the 2026-05-25 diagnostic pattern.

## Changelog

- 2026-05-27 (orchestrator): file created; Bug #64 post-ship UX audit; root-cause
  hypothesis D (preload-sentinel-no-tick + post-engine-instant-clear); two
  scoped fix options (D.1.1, D.2.1) documented but **not applied** pending
  operator confirmation via the 6-section repro recipe.
