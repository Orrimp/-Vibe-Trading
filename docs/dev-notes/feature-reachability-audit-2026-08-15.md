# Feature-reachability audit — 2026-08-15

**Scope**: every `[features]` entry declared by the 17-crate Rust workspace, enumerated
mechanically. Read-only survey; no source, manifest, or `evidence/` file was modified.

**The question**: not "is this feature on?" but the discriminator that bug-log **#81**
taught — *when the feature is OFF, does the `cfg(not(...))` arm fail loudly, or does it
return a value a caller cannot distinguish from a legitimate runtime state?*

`backtest/candle` is enabled nowhere and is nearly harmless: all three of its off-arms
`bail!` with the exact rebuild command. `backtest/realdata` is enabled nowhere and returns
a bare `None`: callers read that as "no data available", the system degrades silently, and
a ranked strategy arm stops working without a single log line. Same enablement, opposite
severity. **The off-path is the finding; the enablement is only the precondition.**

## Method and evidence grade

Every row is tagged:

| tag | meaning |
|---|---|
| **[CARGO]** | the resolver's own answer — `cargo metadata` / `cargo tree`, command quoted |
| **[SRC]** | read at the cited absolute path + line |
| **[GREP]** | exhaustive grep whose full output I inspected (a zero-hit result IS the evidence) |
| **[UNVERIFIED]** | stated because it matters; I could not close it — see §6 |

Manifest reading alone was never accepted where the resolver could disagree. The commands
that produced the enablement columns, verbatim, all run from the repo root:

```
cargo metadata --no-deps --format-version 1          # → 24 features across 6 crates
cargo tree --offline -p ui       -e normal --format "{p} {f}"
cargo tree --offline -p agent    -e normal --format "{p} {f}"
cargo tree --offline -p backtest -e normal --format "{p} {f}"
cargo tree --offline -p backtest --features realdata,candle -e normal --format "{p} {f}"
cargo tree --offline -p forecast -e normal --format "{p} {f}"
cargo tree --offline -p ui       -e features --format "{p} {f}"   # incl. dev-deps
```

Raw results, workspace crates only (`(empty)` = no features at all):

```
-p ui       -e normal : ui=binance,default,live,yahoo · backtest=(empty) · agent=(empty)
                        audit=(empty) · data=yahoo,yahoo-online · forecast=candle,default
                        strategy=forecast
-p agent    -e normal : agent=(empty) · backtest=(empty) · audit=(empty) · data=yahoo
                        forecast=candle,default · strategy=forecast
-p backtest -e normal : backtest=(empty) · data=(empty) · forecast=candle,default
                        strategy=forecast
-p backtest --features realdata,candle : backtest=candle,realdata · strategy=forecast
-p forecast -e normal : forecast=default              (candle is OFF when built alone)
-p ui       -e features (dev-deps included) : data=fixtures,yahoo,yahoo-online
```

**No `cargo build` or `cargo check` was run.** Every finding below rests on resolver output
plus source reading. Where a claim would need a compile or a run to close, it is marked.

**Relationship to the two other 2026-08-15 notes in this directory.** `reachability-map-2026-08-15.md`
(untracked, same date) already publishes a 24-feature enablement matrix and flags
`backtest/candle` (its N-7) and the `forecast/audit-tick` chain (its N-8). I re-derived the
enablement column independently and **it agrees on all 24 rows** — recorded in §3 as a
cross-check. What is new here is the **off-path classification for every feature**, which
that map does not carry, and the four candidates in §1 that fall out of it. Where a finding
extends or sharpens one of its entries, that is stated in the row.

---

## 1. Executive summary — new defect candidates, severest first

Four candidates. None duplicates bug-log #81 or its `realdata` twin (both re-confirmed
mechanically in §3 instead).

Classification key, applied to the `cfg(not(feature))` arm:
**(a)** `bail!`/`panic!`/`compile_error!` — loud, low risk ·
**(b)** returns `None`/empty/default/`Ok`-with-nothing — **silent, high risk** ·
**(c)** item does not exist and every caller is gated too ·
**(d)** no `cfg(not(...))` arm at all — the capability is simply skipped.

---

### NEW-A — HIGH. A documented operator config flag is **silently inert in every build**: `[strategies.tcn_overlay_momentum] enabled = true` cannot register its strategy, because the registration is compile-gated on a three-crate feature chain nothing enables — and the off-arm is `let _ = ledger;`

**Off-path class: (d), and it is the worst version of (d) — not one warning, not one log line.**

Evidence chain, every link verified:

1. **[SRC]** `crates/agent/src/runtime.rs:260` — inside `build_registry_with_ledger`, the
   entire TCN-overlay registration block is `#[cfg(feature = "forecast-audit-tick")]`. It is
   the block that reads `cfg.strategies.tcn_overlay_momentum.enabled`.
2. **[SRC]** `crates/agent/src/runtime.rs:298-299` — the off-arm, in full:
   ```rust
   #[cfg(not(feature = "forecast-audit-tick"))]
   let _ = ledger;
   ```
   No `bail!`, no `warn!`, no `tracing` line. The function's own doc at `runtime.rs:236-238`
   states the consequence and treats it as acceptable: *"when the feature is absent the TCN
   arm is elided and the function degrades to an exact copy of `build_registry`."*
3. **[SRC]** `crates/agent/src/main.rs:233` — the headless `trading` binary calls
   `build_registry_with_ledger(&cfg, (*ledger).clone())`. This is the shipped consumer.
4. **[CARGO]** `cargo tree --offline -p agent -e normal --format "{p} {f}"` → `agent=(empty)`.
   The feature is off in the agent's own build.
5. **[GREP]** `grep -rn 'agent/forecast-audit-tick' --include=Cargo.toml .` → **0 hits.**
   Nothing in the workspace enables it. `agent/Cargo.toml:16` declares it as a pass-through
   to `strategy/forecast-audit-tick` → `forecast/audit-tick`; the whole chain is dark.
6. **[GREP]** `crates/ui/Cargo.toml`'s `[features]` section declares **no** pass-through —
   so the cockpit cannot enable it even deliberately. Only `cargo … -p agent --features
   forecast-audit-tick` can, and **no documented run command does** ([GREP] over `README.md`,
   `docs/runbooks/*.md`, `.github/workflows/ci.yml`, `.claude/launch.json` → 0 hits).
7. **[SRC]** `crates/agent/src/config.rs:487-490` — the knob is a real, deserialised,
   documented config field (`StrategiesConfig::tcn_overlay_momentum`), default `false`, with
   two unit tests at `config.rs:1631` and `:1654` asserting the default and asserting that
   `enabled = true` **parses**. Nothing asserts it takes effect.

**Why this is the #81 shape and not merely an unused opt-in.** The operator surface is a
config file, not a `--features` flag. Setting `enabled = true` produces a config that parses,
validates, passes its tests, and does nothing — with no diagnostic distinguishing it from
"the strategy ran and had no opinion". The severity discriminator is satisfied on both legs:
the off-path is indistinguishable from a legitimate state, and the capability is
operator-visible.

**Relation to the existing record.** `reachability-map` N-8 covers the same feature chain
from the audit-event end (`AuditEvent::ForecastEmitted` has no live producer). The
strategy-registration consequence — an operator-settable config flag that is inert — is not
in N-8 and is the sharper statement, because the audit event is an internal artifact while
the config flag is a documented control.

**Not a crown/anchor risk** [SRC]: this is the paper-mode forward registry, not the bake-off
ranking path, and `write_report` is not involved. The harm is a control that lies.

---

### NEW-B — MEDIUM. `ui/live`'s off-path reports a **successful backtest that never ran**, while its two siblings in the same crate return `Err` for the identical situation

**Off-path class: (b) — silent, plausible value.** Contained today only because `live` is in
`ui`'s `default`; the shape is the defect.

**[SRC]** `crates/ui/src/lab/runner.rs:1274-1291`, the `#[cfg(not(feature = "live"))]` block
of `spawn_lab_run`:

```rust
let summary = RunSummary {
    strategy_id: strategy,  symbol,  report_path: None,
    equity_series: Vec::new(),  fills: Vec::new(),
    kpis: backtest::BacktestKpis::default(),
    bars: Arc::new(Vec::new()),  position_curve: Vec::new(),
};
iced::Task::done(Message::LabRunCompleted(Ok(summary)))
```

`Ok`, not `Err`. Zero fills, zero equity, default KPIs — the exact wire shape of a real run
that produced nothing. The two siblings handle the same case the opposite way:

| function | file:line | `cfg(not(live))` behaviour | class |
|---|---|---|---|
| `spawn_lab_run` | `crates/ui/src/lab/runner.rs:1291` | `Ok(RunSummary{ empty })` | **(b)** |
| `spawn_bakeoff` | `crates/ui/src/leaderboard/runner.rs:242-247` | `Err(LEADERBOARD_RUN_NEEDS_LIVE)` | (a) |
| `spawn_training_run` | `crates/ui/src/lab/trainer.rs:363-372` | `Err("training not supported in non-live fixture builds")` | (a) |

**Reachability, stated honestly.** **[CARGO]** `ui`'s `default = ["live","yahoo","binance"]`,
so every documented command and every CI job builds with `live` **on** — including
`--features fixtures`, which *adds* to defaults rather than replacing them. So no shipping
build takes this branch today. But **[GREP]** `--no-default-features` is explicitly
documented as supported at `crates/ui/Cargo.toml:231-232` (*"…and then re-enable individual
features as needed"*), and historical tester reports under `evidence/v1/` record repeated use
of `cargo test -p ui --no-default-features --features live`. One flag removes the guard.

**A second instance of the same shape is on a RUNTIME branch inside the live build.**
**[SRC]** `crates/ui/src/lab/runner.rs:1297-1310`: when `rt_handle` is `None`,
`spawn_lab_run` returns the same empty `Ok(summary)` — in the shipped cockpit. Whether
`cockpit_live` can ever pass `None` there I did **not** determine (§6.2).

> **RESOLVED 2026-08-15 (orchestrator) — it cannot, and this sub-claim is WITHDRAWN.**
> `cockpit_live.rs:130` declares `rt_handle: tokio::runtime::Handle` — **not** an `Option` —
> built at `:435` from `agent_runtime.handle().clone()` and stored at `:775`; the sole
> production caller (`:2264`) passes `Some(&self.rt_handle)` unconditionally. The `None` arm is
> unreachable from every shipping path, so the shipped cockpit **cannot** report an empty run as
> a success. The remaining `cfg` arm is real but latent — no build compiles it (`live` is a
> default and `--features fixtures` is additive; confirmed against cargo's resolver, not the
> manifests). Logged as bug-log **#91** at **LOW** severity, where the finding that survives is
> a dated one: the stub's rationale landed **2026-05-24** and `live` became default
> **2026-05-25**, so the comment has defended a one-day-old build configuration ever since.

---

### NEW-C — MEDIUM. The feature named `fixtures` does **not** gate the fixture code. `ui`'s fake-cockpit builders and the test-only cockpit factory are compiled unconditionally into the shipped `cockpit_live` binary

**Off-path class: (c), inverted — the item exists whether or not the feature does.**

**[SRC]** `crates/ui/src/lib.rs:113`, `:121`, `:132` — `pub mod fixtures;`,
`pub mod test_support;`, `pub mod gallery;` all carry **no** `#[cfg]`. The in-file rationale
is explicit and deliberate (`lib.rs:109-112`, `test_support.rs:10-22`): integration tests in
`crates/ui/tests/*.rs` see only the public API and cannot import `#[cfg(test)]` items, and
requirement H5 wants them runnable under plain `cargo test -p ui`.

What `ui/fixtures` actually gates: **[SRC]** the `cockpit` and `ui-gallery` bin targets
(`crates/ui/Cargo.toml:20`, `:28` `required-features = ["fixtures"]`), a handful of
`#![cfg(feature = "fixtures")]` test files, and exactly one production line —
`crates/ui/src/bin/cockpit.rs:171`, which seeds the demo cockpit from
`ui::fixtures::fake_cockpit_v15a_pairs_steady_state()`.

**Checked, and clean on the load-bearing question** [GREP]: every `fixtures::fake_*` call
site in `crates/ui/src/` is inside a `#[cfg(test)]` block (`state.rs:6561-6563`,
`screens/tune.rs:1164-1168`, `assistant/view.rs:436`), inside the fixtures-gated `cockpit`
bin, or inside `gallery/routes.rs` / `test_support.rs` — both of which are themselves only
entered from the fixtures-gated gallery bin or from tests. **No shipped cockpit path renders
fixture data.**

**Why it is still a finding.** The naming asserts a guarantee the build does not provide. A
reviewer checking "can fake data reach the operator?" by reading the feature list will
conclude the fixture code is absent from `cockpit_live`. It is present, linked, and one
non-`cfg(test)` call site away from being reachable — and the compiler will not object,
because the module is `pub` in a library crate and `dead_code` is structurally blind to that.

**Companion, same shape, `#[cfg(not(test))]` flavour** **[SRC]**
`crates/ui/src/widgets/chart_legend.rs:439-443`: `LEGEND_CARD_RADIUS_PX` is declared twice —
once `#[cfg(test)]`, once `#[cfg(not(test))] #[allow(dead_code)]` with an identical value.
The production copy exists solely to keep the symbol present and is never read. Harmless
(it is `radius::R3`), recorded because it is the same "test scaffolding in the production
build" family.

---

### NEW-D — LOW. `audit/test-support` gates a helper with **zero callers anywhere in the workspace** — including under `cfg(test)`

**Off-path class: (c) — harmless.** Recorded because a feature whose entire payload is
unreferenced is a maintenance trap: the next reader assumes tests depend on it.

**[SRC]** `crates/audit/src/ledger.rs:112-120` — `#[cfg(any(test, feature = "test-support"))]
pub fn with_pid(&self, pid: u32) -> Self`, doc-commented *"Test helper — override the
pre-seeded `agent_pid` for deterministic assertions in tick tests."*
**[GREP]** `grep -rn 'with_pid' --include='*.rs' crates/` → **one hit, the definition itself.**
No tick test, no integration test, nothing calls it.
**[CARGO]** `audit=(empty)` in the `-p ui`, `-p agent` and `-p backtest` resolutions, and
**[GREP]** `grep -rn 'audit/test-support' --include=Cargo.toml .` → 0 hits.

---

### Also new, sub-defect, recorded for completeness

**NEW-E — `backtest/candle` is a no-op dependency edge.** **[SRC]**
`crates/backtest/Cargo.toml:36` declares `candle = ["strategy/forecast"]`, but line 63
already sets `strategy = { path = "../strategy", features = ["forecast"] }` unconditionally.
**[CARGO]** `-p backtest -e normal` → `strategy=forecast`; `-p backtest --features
realdata,candle -e normal` → `strategy=forecast`. Identical. Turning `backtest/candle` on
adds nothing to `strategy`; it only flips `backtest`'s own `#[cfg]` blocks. (`reachability-map`
N-7 records that the feature is enabled nowhere; that its declared payload is already
satisfied is new.)

**NEW-F — the Metal/CPU numerical-drift guard executes in no build on any OS, and its
skip-arm is a passing test.** **[SRC]** `crates/forecast/tests/metal_cpu_drift.rs:28` is
`#[cfg(all(feature = "metal", target_os = "macos"))]`; the off-arm at `:126` is
`fn metal_cpu_drift_not_applicable()` — a `println!` that always passes, comment *"On
CPU-only CI it is a no-op pass."* **[CARGO]** `forecast/metal` is off in every resolution
above, so the macOS CI job runs the stub too. The guard is green everywhere and has never
compared anything. (This is the repo's own vacuous-test class — bug-log #66 — on a
`cfg(target_os)` surface.)

---

## 2. The full 24-feature matrix

**[CARGO]** 24 features across 6 crates, from `cargo metadata --no-deps --format-version 1`
(the 11 remaining crates declare none). Columns: **Cockpit** = `-p ui` default (the shipped
`cockpit_live`/`viewer`) · **Agent** = `-p agent` (`trading` bin) · **BT-lib** = `-p backtest`
default · **BT-doc** = `-p backtest --features realdata,candle` (the README research build) ·
**CI** = any `.github/workflows/ci.yml` job. ✅ on · ❌ off · *n/a* feature not declared there.

| # | feature | gates what | Cockpit | Agent | BT-lib | BT-doc | CI | `cfg(not(...))` behaviour | class | sev |
|---|---|---|:--:|:--:|:--:|:--:|:--:|---|:--:|:--:|
| 1 | `agent/forecast-audit-tick` | TCN-overlay registration in `build_registry_with_ledger` + the `ForecastEmitted` tee | ❌ | ❌ | *n/a* | *n/a* | ❌ | `let _ = ledger;` — arm elided, **no log** (`runtime.rs:298`) | **(d)** | **HIGH** |
| 2 | `agent/in_process_cron` | whole module `agent/src/cron.rs` (`#![cfg]` floor-gate) + scheduler spawn `runtime.rs:894` | ❌ | ❌ | *n/a* | *n/a* | ❌ | no not-arm; step 5 of `run()` is skipped silently — weekly operator reports never scheduled | (d) | LOW¹ |
| 3 | `audit/test-support` | `Ledger::with_pid` (`ledger.rs:112`) | ❌ | ❌ | ❌ | ❌ | ❌ | item absent; **zero callers even under `cfg(test)`** | (c) | LOW → **NEW-D** |
| 4 | `backtest/candle` | `scenarios/{tcn_overlay_weights,patchtst_overlay_weights,threshold_sweep}::run` | ❌ | ❌ | ❌ | ✅ | ❌ | `anyhow::bail!` × 3, each naming the exact rebuild command (`:37`, `:52`, `:65`) | **(a)** | LOW² |
| 5 | `backtest/realdata` | `realdata`/`dvol_data`/`basis_data`/`funding_data` modules + `resolve_dvol_override` + real-data scenarios | ❌ | ❌ | ❌ | ✅ | ❌ | **mixed** — see the split below | (a)+**(b)** | known (#81 twin) |
| 6 | `backtest/yahoo` | `macro_regime` module (`#![cfg]` floor-gate) + the macro preload in `run_bakeoff` | ❌ | ❌ | ❌ | ❌ | ❌ | `preloaded_macro_series = None` (`bakeoff/mod.rs:1158`) | **(b)** | known (#81) |
| 7 | `data/fixtures` | `mock_feed` module + `MockFetcher`/`make_kline`/`make_batch` | ❌ prod / ✅ dev-dep | ❌ | ❌ | ❌ | ✅ | items absent; every caller is a test or fixtures-gated | (c) | none |
| 8 | `data/yahoo` | whole module `data/src/yahoo.rs` (`#![cfg]` floor-gate) | ✅ | ✅ | ❌ | ❌ | ✅ | in `ui`: `Err("…requires the `yahoo` feature; rebuild with --features yahoo")` (`lab/runner.rs:1327`) | **(a)** | none |
| 9 | `data/yahoo-online` | `fetch_and_cache` + 7 online paths in `yahoo.rs` | ✅ | ❌ | ❌ | ❌ | ✅ | no not-arm; items absent, `fetch_yahoo_klines` bin `required-features`-gated | (c) | LOW³ |
| 10 | `forecast/audit-tick` | 13 tee sites in `tcn.rs`, 2 in `patchtst.rs` | ❌ | ❌ | ❌ | ❌ | ❌ | no not-arm; the audit tee simply never fires | (d) | see NEW-A |
| 11 | `forecast/candle` | `FeatureTensor = Tensor`, TCN/PatchTST inference | ✅ | ✅ | ✅ | ✅ | ✅ | `FeatureTensor = Vec<f32>` (`features.rs:234`) — a real typed fallback, not a stub | (b)-shaped⁴ | none |
| 12 | `forecast/default` | `default = []` — declares nothing, gates nothing | ✅ | ✅ | ✅ | ✅ | ✅ | *n/a* | — | none |
| 13 | `forecast/metal` | Apple-GPU device selection in `train_tcn`/`train_patchtst`; the drift test | ❌ | ❌ | ❌ | ❌ | ❌ | `let device = Device::Cpu;` (`train_tcn.rs:461`) — silent, but the on-path also falls back to CPU with a `warn!` | (b) | LOW → **NEW-F** |
| 14 | `strategy/forecast` | `TcnSyncForecaster`, the candle strategy surface, `mod checkpoint_loader` | ✅ | ✅ | ✅ | ✅ | ✅ | items absent; enabled unconditionally by `backtest/Cargo.toml:63` | (c) | none |
| 15 | `strategy/forecast-audit-tick` | pass-through to `forecast/audit-tick` | ❌ | ❌ | ❌ | ❌ | ❌ | *n/a* (pure propagator) | — | see NEW-A |
| 16 | `ui/binance` | Binance Lab source chip | ✅ | *n/a* | *n/a* | *n/a* | ✅ | `Err("Binance data source requires the `binance` feature; rebuild with --features binance")` (`lab/runner.rs:1341`) | **(a)** | none |
| 17 | `ui/chart-build-probe` | geometry-build timer + `pub` vs `pub(crate)` on the module | ❌ | *n/a* | *n/a* | *n/a* | ❌ | zero-sized no-op `BuildTimer` shim (`chart_build_probe.rs:116-126`) — silent **by design**, identical call shape | (b) | none⁵ |
| 18 | `ui/default` | `["live","yahoo","binance"]` | ✅ | *n/a* | *n/a* | *n/a* | ✅ | *n/a* | — | none |
| 19 | `ui/fixtures` | the `cockpit` + `ui-gallery` **bins** and ~6 test files — **not** the `fixtures` module | ❌ | *n/a* | *n/a* | *n/a* | ✅ | `Cockpit::new()` at `bin/cockpit.rs:172` — **unreachable**, that bin is `required-features = ["fixtures"]` | (c)/dead | MED → **NEW-C** |
| 20 | `ui/in_process_cron` | pass-through to `agent/in_process_cron` (forces `live`) | ❌ | *n/a* | *n/a* | *n/a* | ❌ | *n/a* (pure propagator) | — | LOW¹ |
| 21 | `ui/live` | `live.rs` + `forward_plan/adapter.rs` (`#![cfg]` floor-gates), all async spawns, ~101 sites in `cockpit_live.rs` | ✅ | *n/a* | *n/a* | *n/a* | ✅ | **mixed** — 2 sites `Err`, 1 site `Ok(empty)` | (a)+**(b)** | MED → **NEW-B** |
| 22 | `ui/record-tests` | `iced/{tester,selector,strict-assertions}` + one `info!` at `cockpit_live.rs:839` | ❌ | *n/a* | *n/a* | *n/a* | ❌ | no not-arm; recorder overlay absent | (c) | none |
| 23 | `ui/render-debug` | `widgets/debug_renderer.rs` (`#![cfg]` floor-gate) + probes at `bin/cockpit.rs:127`, `widgets/strategies.rs:224` | ❌ | *n/a* | *n/a* | *n/a* | ❌ | no not-arm; diagnostic absent | (c) | none |
| 24 | `ui/yahoo` | `dep:data` + `data/yahoo` + `data/yahoo-online` | ✅ | *n/a* | *n/a* | *n/a* | ✅ | see #8 — `Err` with rebuild command | **(a)** | none |

¹ `agent/in_process_cron` is a **documented opt-in** (`cron.rs:4-12`, `runtime.rs:800`,
`agent/Cargo.toml:66`) and `ui`'s pass-through is correctly wired, so the off-state is
intended. Recorded because the consequence — weekly operator success reports are never
scheduled on a fresh checkout — is silent, and was already logged as G5 in
`docs/dev-notes/archive/2026-Q2/paper-trade-live-infra-audit-2026-05-29.md`.

² `backtest/candle` is the **reference specimen for the good shape**: enabled nowhere, and a
developer who reaches it hits a wall in one line. This is why enablement alone is not the
severity axis.

³ Cross-build divergence worth knowing: **[CARGO]** `-p agent` resolves `data=yahoo` but
`-p ui` resolves `data=yahoo,yahoo-online`. A workspace-wide build unifies them; a
per-package build does not. The agent's `data` therefore has the parquet reader but not the
online fetcher.

⁴ `forecast/candle`'s off-path swaps `FeatureTensor` from `Tensor` to `Vec<f32>` and the
pipeline keeps working on the plain vector — technically a silent substitution, but the type
change is total and compiler-enforced at every use site, so a caller cannot be handed a
degraded value while believing it got a tensor. It is also ✅ everywhere. **[CARGO]** note:
`cargo test -p forecast` **alone** resolves `forecast=default` with candle **OFF**, which
silently skips all 13 `required-features = ["candle"]` targets in `crates/forecast/Cargo.toml`.
CI's `cargo test --workspace --exclude ui` unifies candle on, so this bites only a developer
running the crate in isolation.

⁵ `ui/chart-build-probe`'s off-arm is a deliberate zero-sized no-op so the chart `draw`
impls carry one unconditional line. Class (b) by letter, benign by consequence: the
capability is a timing probe, so "no timing recorded" is the honest answer and no decision
reads it.

### `backtest/realdata` — the mixed row, split by site

The crate is **not** uniform, and the split is exactly the severity axis. **[SRC]**, all 13
`cfg(not(feature = "realdata"))` sites in `crates/backtest/src/` (a 14th grep hit,
`bakeoff/mod.rs:275`, is a doc comment):

| site | off-path | class |
|---|---|---|
| `src/bakeoff/mod.rs:377` `resolve_dvol_override` | `None` — **the #81 twin** | **(b)** |
| `src/main.rs:309` `build_volume_map_for_scenario` | `None` | (b), see note |
| `src/main.rs:211` slippage-model fallback | `Linear{bps:8}` **+ `info!` line** | (b)/logged |
| `src/main.rs:1356`, `:1456` real-data scenario dispatch | `return Err(…)` / `bail!` naming the rebuild command | (a) |
| `src/bin/monte_carlo.rs:388`, `:659` | `bail!` × 2 | (a) |
| `src/bin/threshold_sweep.rs:698` | `bail!` | (a) |
| `src/bin/param_robustness_sweep.rs:546`, `:1106`, `:1258`, `:1495`, `:1724` | `bail!` × 5 | (a) |

**`main.rs:309` was checked and is NOT a defect** — stated because it looks like one. The
volume map feeds the square-root slippage model, which is only used by real-data scenarios
(`main.rs:187-206`); in a non-realdata build those scenarios `bail!` at `:1356`/`:1456` before
the map is consulted, and the synthetic path takes the `Linear{bps:8}` fallback that logs at
INFO. The `None` is consistent with the only reachable state. **10 of 13 sites `bail!`, 1
more logs its substitution, and the single dangerous one is the already-known
`resolve_dvol_override`.**

---

## 3. Self-test — does this method reproduce the two known positives?

**Yes, both, on resolver evidence alone, before any source was read.** Run as a blind
detector over the enumerated feature list:

| step | `backtest/yahoo` (#81) | `backtest/realdata` (the twin) |
|---|---|---|
| declared? | ✅ `backtest/Cargo.toml:35` | ✅ `:36` |
| in its crate's `default`? | ❌ — **[GREP]** `grep -c '^default' crates/backtest/Cargo.toml` → **0** (no `default` stanza at all) | ❌ same |
| enabled by any manifest? | ❌ — **[GREP]** `grep -rn 'backtest/yahoo' --include=Cargo.toml .` → **0 hits** | ❌ `'backtest/realdata'` → **0 hits** |
| resolver's verdict for the shipped cockpit | ❌ — **[CARGO]** `cargo tree --offline -p ui -e normal` → `backtest=(empty)` | ❌ same, same command |
| resolver's verdict for the headless agent | ❌ — `-p agent -e normal` → `backtest=(empty)` | ❌ same |
| enabled by any documented command? | ❌ — only the `run_yahoo_sma` **bin**'s `required-features`, which propagates nothing | ✅ **only** by `--features realdata` on the research CLI; ❌ for every product command |
| off-path | `preloaded_macro_series = None` (`bakeoff/mod.rs:1158`) → **(b) silent** | `resolve_dvol_override → None` (`bakeoff/mod.rs:377`) → **(b) silent** |
| **flagged?** | **YES** | **YES** |

The `required-features` trap the brief warns about reproduced too, and the resolver refuted
it: **[SRC]** `crates/backtest/Cargo.toml` carries `required-features = ["realdata"]` on two
bins and one example and `["yahoo"]` on `run_yahoo_sma`, which reads like the features are in
use. **[CARGO]** `-p ui -e normal` → `backtest=(empty)` regardless. Manifest reading says
"used"; the resolver says "not for any library consumer". The same trap hides `ui/fixtures`
(NEW-C) and gated the `cockpit` bin's now-unreachable `cfg(not(fixtures))` arm.

**Cross-check against `reachability-map-2026-08-15.md` §3**: its enablement column and mine
were derived independently (it used `cargo tree -e normal` + `cargo metadata`; I used the
per-entry-point commands quoted in the header). **All 24 rows agree.** Its "24 declared
features" count also matches my `cargo metadata` enumeration exactly, confirming the
earlier hand-counted "seven" in bug-log #81's extension table was the undercount the brief
describes.

---

## 4. The `#[allow(dead_code)]` sample — stale vs live

**[GREP]** `grep -rn 'allow(dead_code)' --include='*.rs' crates/*/src/ | wc -l` → **42**,
matching the prior census. Sampled 20; verdicts below. **"STALE"** = the suppression covers
an item that is in fact alive, so the annotation is misinformation. **"VALID"** = the item
really is unreferenced, and the annotation is load-bearing (and sometimes marks a real
inertness defect).

| # | site | item | verdict | evidence |
|---|---|---|---|---|
| 1 | `crates/ui/src/lab/state.rs:174` | `training_inflight` | **STALE** | read at `bin/cockpit_live.rs:1752`, `screens/lab.rs:897`; written at `state.rs:3385`, `:3392`, `cockpit_live.rs:1781` |
| 2 | `crates/ui/src/bin/cockpit_live.rs:1125` | `trail_mirror_handle` | **STALE** | read at `cockpit_live.rs:1322`, `:2391`, `:2417` (the in-line comment "accessed via subscription() only" is itself the tell) |
| 3 | `crates/ui/src/widgets/axis.rs:94` | `y_for_value` | **STALE for the lint, but production-dead** | referenced only at `axis.rs:173-174`, inside `#[cfg(test)]`. Alive to the compiler, dead to the product — a unit-tested helper nothing calls |
| 4 | `crates/ui/src/widgets/chart.rs:1246` | `strategy_label_or_none` | **VALID** | **[GREP]** zero references outside the definition |
| 5 | `crates/ui/src/widgets/positions.rs:193` | `warn_if_over` | **VALID** | zero references outside the definition |
| 6 | `crates/ui/src/widgets/chart_legend.rs:442` | `LEGEND_CARD_RADIUS_PX` | **VALID by construction** | `#[cfg(not(test))]` twin of the `#[cfg(test)]` const at `:440`; the production copy exists only to keep the symbol defined (see NEW-C companion) |
| 7 | `crates/strategy/src/cross_sectional/momentum.rs:38` | `drift_threshold` | **VALID — and it marks a real defect** | written from config at `:194`, never read anywhere. Independently confirms the drift-rebalance knob is inert |
| 8 | `crates/backtest/src/paper.rs:68` | `rng: ChaCha20Rng` | **VALID — and it marks a real defect** | constructed at `:85` from the seed; **[GREP]** `grep -n 'rng' crates/backtest/src/paper.rs` returns exactly those two lines. The `PaperEngine` seed cannot affect anything |
| 9 | `crates/strategy/src/vol_targeting_overlay.rs:763` | `mod checkpoint_loader` | **VALID** | zero references. Note it is `#[cfg(feature = "forecast")]`, and **[CARGO]** `strategy/forecast` is ✅ in every build — so a dead GARCH-checkpoint loader compiles into the shipped cockpit |
| 10 | `crates/forecast/src/features.rs:120` | `VolTargetKind::RealizedVol` | **VALID** | matched at `:800` but never constructed; the enum derives no `Deserialize`, so config cannot produce it → the `unimplemented!()` at `:801` is **unreachable**. Checked negative |
| 11 | `crates/reflection/src/audit_tick_consumer.rs:24` | `store: Arc<S>` | **VALID** | annotated in place *"kept for the follow-up brief that writes lessons"* — honest, self-describing |
| 12-14 | `crates/data/src/coinbase.rs:74`, `:99`, `kraken.rs:79` | serde wire fields (`product_id` ×2, `symbol`) | **VALID** | deserialisation targets, never read — the ordinary and correct use of the annotation |
| 15 | `crates/forecast/src/features.rs:272` | `pub open: f64` | **VALID** | struct field carried for shape completeness |
| 16-17 | `crates/agent/src/plan.rs:694`, `crates/ui/src/assistant/view.rs:430` | `_smol_str_smoke`, `_message_used` | **VALID** | one-line import-liveness smoke fns; leading `_` signals intent |
| 18-20 | `crates/backtest/src/main.rs:167`, `:325`, `:370` etc. | `cfg_attr(not(feature="realdata"), allow(dead_code))` × 6 | **VALID and exemplary** | these are *conditional* — the suppression exists only in the build where the item genuinely is dead. This is the pattern the other 36 should follow |

**Sample verdict: 3 of 20 stale (15%)**, 2 more (#7, #8) valid but marking genuine
inertness defects, and 1 (#3) production-dead-but-test-alive. Scaled to 42 that is roughly
6 stale — which independently matches the count in `reachability-map` §4.5, though our
specific lists differ (it names `agent/runtime.rs:120/125/178`, `pairs/config.rs:157` and
`cockpit_live.rs:1104`, which I did not re-sample; I add `axis.rs:94` as production-dead).

---

## 5. `cfg(target_os)` findings

**[GREP]** the complete inventory across the workspace.

**In production `src/` — 5 sites, all safely paired:**

| site | gate | pairing |
|---|---|---|
| `crates/ui/src/lab/pid_alive.rs:43` / `:66` | `#[cfg(unix)]` / `#[cfg(windows)]` | complete pair — every target is covered |
| `crates/ui/src/lab/persistence.rs:106` | `#[cfg(windows)]` | additive Windows-only branch |
| `crates/ui/src/lab/trainer.rs:394`, `:430` | `#[cfg(unix)]` | inside the `live` training path |
| `crates/ui/Cargo.toml:169` | `[target.'cfg(windows)'.dependencies]` | additive |

No production capability was found that exists on one OS and is *silently absent* on
another. `unix`/`windows` covers all three CI targets.

**In `tests/` — the finding.** **[GREP]**
`grep -rln '#!\[cfg(target_os = "macos")\]' crates/ui/tests/ | wc -l` → **32 test files** carry a file-floor
`#![cfg(target_os = "macos")]`, including every rendered-pixel harness the AD-10
non-negotiable rests on: `render_snapshots.rs`, `visual_snapshots.rs`, `panel_snapshots.rs`,
`gallery_snapshots.rs`, `leaderboard_populated_render.rs`, `reports_populated_curve_render.rs`,
`forward_plan_populated_render.rs`, `crown_credibility_render.rs`, `trail_drawer_open_render.rs`,
`plan_export_button_render.rs`, the three `_audit_group_{a,b,c}_render.rs`, and the rest.

The consequence, given the 3-OS matrix **[SRC]** `.github/workflows/ci.yml:117,126,135,145,155,164`:

| job | command | pixel suites executed |
|---|---|---|
| macOS | `cargo test -p ui` | **all** — the canonical gate |
| Linux | `xvfb-run -a cargo test -p ui --features fixtures` | **none** — the files compile to empty and the job is green |
| Windows | `cargo test -p ui --features fixtures` | **none** — same |

This is documented and deliberate (ADR-0057 D2, cited in the file headers, and `ci.yml:20-21`
states the macOS job is the canonical gate). It is recorded here because it means **two of
the three CI legs report PASS on the AD-10 surface without executing a single pixel
assertion** — a green tick that carries no rendering evidence. Two files deliberately opt
out of the macOS floor to hold a cross-OS line (`live_kpi_strip_render.rs:48-50` and
`headless_emulator_smoke.rs:51` say so explicitly); they are the only pixel coverage Linux
and Windows have.

**Combined feature × target_os gate — 1 site, and it never runs anywhere:**
`crates/forecast/tests/metal_cpu_drift.rs:28` is `#[cfg(all(feature = "metal", target_os =
"macos"))]`. **[CARGO]** `forecast/metal` is off in every resolution in §2, so even the macOS
job takes the `:126` stub that always passes. See NEW-F.

---

## 6. What I could not determine, and why

A labelled unknown is worth more than a confident guess.

1. **No compile or run evidence backs any row.** Every claim rests on `cargo tree` /
   `cargo metadata` plus source reading. I did **not** run `cargo build`, `cargo check`, or
   any test. Specifically unproven: that flipping `agent/forecast-audit-tick` on (NEW-A)
   actually compiles today — the chain has been dark long enough for bit-rot, and the
   archived command `cargo run --features live,forecast-audit-tick --bin cockpit_live`
   (`evidence/v1/ui-rethink-phase-d-trail-followup/reports/test-final-2026-05-20.md:386`)
   **cannot** work now, because `ui` declares no such feature. That evidence file is
   anchored and byte-immutable; it must not be edited to fix the command.

2. **Whether `rt_handle` can be `None` in the shipped `cockpit_live`** (NEW-B's runtime
   twin, `crates/ui/src/lab/runner.rs:1297`). I verified the branch exists and returns the
   same empty `Ok(summary)`; I did not trace every construction site of the `AppState` that
   supplies it. **[UNVERIFIED]** — needs a read of `cockpit_live.rs`'s runtime setup or a run.

3. **Whether any consumer distinguishes an empty `RunSummary` from a real one.** NEW-B's
   severity depends on what the Lab screen renders for `equity_series: vec![]` — an honest
   "no data" panel would blunt it, a "run complete" toast would sharpen it. Not traced.

4. **`data/fixtures` under a workspace-wide build.** **[CARGO]** `-e normal` shows it off
   for production and `-e features` (dev-deps included) shows it on, which is the correct
   separation for `-p ui`. Whether `cargo build --workspace` (no `--tests`) can unify it into
   a production artifact through some other member's dependency I did **not** establish;
   `-e normal` says no for every entry point I tested, but I tested five, not all seventeen.

5. **The 22 `#[allow(dead_code)]` sites I did not sample**, and whether the 15% stale rate
   holds. The sample was chosen for spread across crates, not randomly.

6. **Runtime-gated capabilities are out of scope here.** This audit covers `cfg(feature)`
   and `cfg(target_os)`. A capability disabled by an `if cfg.enabled` that is never true is
   the same defect class — NEW-A is a *hybrid* (compile-gated block reading a runtime flag),
   found only because the cfg trail led there. A systematic runtime-flag sweep has not been
   done.

7. **Anchor impact: not assessed, and not incurred.** Several files named above
   (`scenarios/*.rs`, `bin/threshold_sweep.rs`) do write anchored bodies. **This document
   changes no code and touches no `evidence/` file**, so the 119-anchor gate is unaffected
   by writing it. Any fix arising from §1 must run `bash scripts/verify_anchors.sh` before
   and after.

8. **This is a snapshot of a tree under active edit, and one file moved under me.**
   `crates/backtest/src/bakeoff/mod.rs`, `crates/ui/src/leaderboard/runner.rs`,
   `.github/workflows/ci.yml` and `crates/agent/src/runtime.rs` are all modified in the
   working tree as of this pass. **`crates/agent/src/runtime.rs` was rewritten by a
   concurrent session mid-audit** (`git diff --stat` → 210 insertions, 93 deletions). I
   re-checked NEW-A against the post-edit file: the gate at `:259`, the off-arm at `:298`,
   and the doc sentence at `:236` all survive unchanged, so **NEW-A is current**. Nothing
   else was re-checked after that edit. Line numbers cited for `ci.yml` are from the
   working tree, not `HEAD` — they differ from those in `reachability-map-2026-08-15.md`
   for that reason. Re-run the commands in the header before acting on any row.

---

## Appendix — the detector, reusable

```bash
# 1. Enumerate every declared feature. Never count these by hand.
cargo metadata --no-deps --format-version 1 \
  | python3 -c "import json,sys; m=json.load(sys.stdin); [print(p['name'],k,'=',v) for p in m['packages'] for k,v in sorted(p.get('features',{}).items())]"

# 2. For each ENTRY POINT, ask the resolver what each crate actually gets.
#    -e normal excludes dev-deps; without it, test-only features look production-enabled.
cargo tree --offline -p <entrypoint> -e normal --format "{p} {f}"

# 3. Does ANY manifest enable <crate>/<feature>?  A zero-hit result IS the finding.
grep -rn '<crate>/<feature>' --include=Cargo.toml .

# 4. THE SEVERITY QUESTION — run this last, and let it decide the ranking.
grep -rn 'cfg(not(feature' --include='*.rs' crates/*/src/
#    bail!/panic!/compile_error!  -> (a) loud, low risk
#    None / vec![] / default() / Ok(empty)  -> (b) SILENT, high risk
#    item simply absent, callers gated too  -> (c) check for an ungated caller
#    no cfg(not(...)) arm at all            -> (d) the capability is skipped in silence
```

Step 4 is the one the earlier passes skipped. Steps 1-3 tell you a feature is off; only
step 4 tells you whether that matters.
