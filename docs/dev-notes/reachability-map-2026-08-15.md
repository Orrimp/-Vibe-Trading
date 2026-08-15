# Reachability map — 2026-08-15

**Scope**: the whole Rust workspace (17 crates, ~772 `.rs` files). Read-only survey.
**Question asked**: not "is this code correct" but **"is this code connected, and what
ships but can never run"**.

**Why**: the 14-story adversarial burn-down produced nine disclosures (bug-log #74–#82).
Not one was a wrong algorithm. Every one was a connectivity failure — a config that never
reached the engine (#79), a module never compiled into any build (#81), one field serving
two purposes (#75), a capability named in a strategy that never happens (#82), a code path
bypassing the shared one (#80). This map generalises the detector.

**Method + evidence grade.** Every row below is tagged:

| tag | meaning |
|---|---|
| **[CARGO]** | verified from `cargo tree -e normal` / `cargo metadata` / a real `cargo build` — the resolver's own answer, not a manifest reading |
| **[SRC]** | verified by reading the source at the cited absolute path + line |
| **[GREP]** | verified by an exhaustive grep whose full output I inspected (a zero-hit result is the evidence) |
| **[INFER]** | reasoned, not directly observed — treat as a hypothesis |
| **[UNVERIFIED]** | stated because it matters, but I could not close it; see §7 |

Manifest reading alone was never accepted where the resolver could disagree.

---

## 1. Executive summary — NEW defect candidates

**22 new defect candidates**, ordered by severity: 2 CRITICAL, 7 HIGH, 8 MEDIUM, 5 LOW,
plus 1 structural finding (N-0) that explains why the rest survived a green build. "New" =
not already recorded in bug-log #74–#82; where a finding extends or **corrects** an existing
entry, that is stated.

Two of them change the disposition of open bug-log entries: **N-2b corrects #82's framing**
(the short slate is not in the shipped field at all), and **N-2 shows #80 is only half

> **UPDATE 2026-08-15 (orchestrator):** N-2 is **CLOSED**. The forward-loop half was fixed in `crates/agent/src/runtime.rs` and gated by `crates/agent/tests/short_long_friction_parity_forward_e2e.rs` (4 assertions, non-vacuous: floor of 10 short-leg fills, 56 measured). Verified independently: FROZEN AD-1 files byte-untouched, anchors 119/119 both sides, long-only control byte-identical. **`try_open_short`/`try_cover_short` now have zero production call sites workspace-wide.** A third exit from the same state — `check_and_liquidate`, which emits no `Fill` and is invisible to both parity gates — is logged as bug-log **#90** and left for an operator decision (it changes *what* is liquidated, not just its cost).

fixed** (the forward paper loop still has the bypass).

---

### N-0 — STRUCTURAL, and it explains the other twenty-two. **The compiler cannot see this defect class here.** `cargo check --workspace` emits ZERO dead-code warnings — not because the lint is disabled, but because rustc is structurally blind to `pub` items in library crates, and the 42 cases it *could* see have each been individually silenced.

**[CARGO]** `cargo check --workspace` completed (exit 0);
`grep -iE "never used|never read|never constructed"` over its full output returns
**nothing**.

**The obvious explanation was checked and is wrong — there is no global suppression.**
All five mechanisms verified negative: no `.cargo/config.toml` exists · root `Cargo.toml`'s
`[workspace.lints.rust]` contains only `unsafe_code = "warn"` · no per-crate `[lints]`
section in any of the 17 manifests · no crate-root `#![allow(dead_code)]` in any `lib.rs` /
`main.rs` · no `RUSTFLAGS` in the environment or any script/CI file. **`dead_code` sits at
its default `warn` in every crate and is fully capable of firing.**

So the silence has two causes, and the second is the load-bearing one:

**(1) 42 targeted `#[allow(dead_code)]` annotations** — **[GREP]**
`grep -rn 'allow(dead_code)' --include="*.rs" crates/*/src/ | wc -l` → **42**
(`ui` 8 · `strategy` 5 · `forecast` 3 · `backtest` 3 · `data` 2 · `reflection` 1 ·
`agent` 1 · remainder file-level or on serde/generated items). These are *source-level proof
the lint does fire*: each exists because somebody saw the warning and muted it in place.
Six are **stale**, covering items that are in fact alive (§4.5).

**(2) — the dominant cause — rustc does not report unused `pub` items in a library crate.**
A `pub fn` in a lib is part of that crate's public API, so it is never "dead" from rustc's
point of view, *regardless of build configuration*. In a 17-crate workspace where nearly
every item is `pub` to cross a crate boundary, this makes the detector close to blind by
construction. Every Tier-1 finding below is invisible to it for this reason alone:
`spawn_heartbeat_monitor`, `trail_for_fill_id`, `sharpe_ratios`, `apply_latency`, and the
entire 289-line `forecast::vol` module are all `pub`.

**Why this is the headline.** Nine consecutive disclosures (#74–#82) were connectivity
failures, and the language's own detector for that class **cannot** cover this codebase's
shape. That is not a lint-configuration problem to be fixed by flipping a setting — it is
the structural reason a green build carries a provider module with zero references, two
strategies reachable only from their own passing e2e tests, an engine seed that cannot
affect behaviour, and eleven config fields written and never read.

**What actually works here** is the method every finding below came from, and it is cheap
enough to automate: **cross-crate identifier-occurrence counting with write-sites separated
from read-sites**. A `pub` item whose occurrence count outside its own definition is zero is
dead no matter what rustc says; a field with writes and no reads is inert no matter how many
validators guard it. All 25 dead-code findings came from that, and **zero** came from the
compiler.

*Caveat, stated rather than papered over*: the confirmation `cargo check` re-compiled only
4 crates (the others were fresh in `target/`), and cargo does not re-emit warnings for fresh
crates. I cannot prove from build output alone that `target/` was cold for all 17 at session
start. This does not weaken the conclusion — cause (1) is source-level evidence independent
of any build, and cause (2) is a property of rustc, not of this run.

---

### N-1 — CRITICAL. **CONFIRMED; PRODUCTION CODE FIXED MID-SWEEP; THE TEST IS STILL STALE AND `cargo test -p ui --lib` IS RED.** The advisor told the operator **19 strategies** were raced; **18** ran.

> **⚠ STATUS NOTE — this entry tracked a moving tree and the timeline is part of the
> evidence.** I found N-1 at ~09:20 and wrote it up at 09:34. Another session then edited
> `crates/backtest/src/bakeoff/mod.rs` at **09:36:27** (its new doc-comment cites *"the
> 2026-08-15 reachability sweep"*; the in-code comment reads *"Added 2026-08-15 (reachability
> sweep, N-1)"*) and `crates/ui/src/leaderboard/runner.rs` at **09:51:03**. **Neither edit is
> mine** — this sweep is read-only, and I re-verified after each change rather than assuming
> my earlier reading still held.
>
> **Current state, verified 09:52:** production code correct on both sides · **test still
> stale** · **`cargo test -p ui --lib` still FAILS** · one transient defect (N-1d) was
> introduced at 09:36 and closed at 09:51. The original finding is preserved at §1e because
> it is the evidence the fix rests on.

#### 1a. The original defect (as found, now fixed in `backtest`)

`arm_runs_in_this_build()` was introduced by the #81 fix so the operator-facing arm count
can never claim an arm the dispatch loop drops. It handled exactly one arm.

#### 1b. Build-verified — the test run I queued came back RED, and the failure IS the finding

`cargo test -p ui --lib leaderboard::runner` (33m50s compile) finished after the edit
landed:

```
running 4 tests
test …::arm_count_drops_the_dvol_arm_for_unsupported_coins ... ok
test …::arm_count_excludes_arms_that_cannot_run_in_this_build ... FAILED

panicked at crates/ui/src/leaderboard/runner.rs:351:13:
v0.dvol_regime must be dispatchable in every build

test result: FAILED. 3 passed; 1 failed
```

That assertion is the one I flagged at 09:34 as encoding a false invariant. The compiler's
own test runner has now confirmed it, from the opposite side: the predicate changed, and the
stale assertion that "every other declared arm is always dispatchable" broke on exactly
`v0.dvol_regime`.

#### 1c. **Production code is now fixed on both sides; THE TEST IS NOT, and the gate is RED**

Verified state as of **09:52** (both files uncommitted; neither edit is mine):

**[SRC]** `crates/backtest/src/bakeoff/mod.rs` (09:36:27, +37 lines) — a new
`dvol_arm_compiled()` returning `cfg!(feature = "realdata")`, wired in:
```rust
pub fn arm_runs_in_this_build(strategy_id: &str) -> bool {
    match strategy_id {
        "v0.macro_riskon" => macro_arm_compiled(),
        "v0.dvol_regime"  => dvol_arm_compiled(),   // added 09:36
        _ => true,
    }
}
```

**[SRC]** `crates/ui/src/leaderboard/runner.rs` (09:51:03, +24 lines) — `advisor_field_arm_count_for`
now guards the coin-time subtraction on the build gate. **This closes N-1d (below), which
was open for ~15 minutes.**

**[SRC] But the test is untouched and still encodes the pre-fix invariant.** It still carries
the loop asserting every non-macro arm is dispatchable, and the terminal literal
`assert_eq!(advisor_field_arm_count(), 19, "shipped build: 18 runnable arms + buy-and-hold")`
— which is now **18**, not 19. Both assertions are false against the code shipping beside
them.

**Consequence: `cargo test -p ui --lib` still FAILS.** The `--lib` suite is part of the macOS
canonical CI gate (`.github/workflows/ci.yml:144`), so this is a **red gate in the working
tree right now**, not a latent issue.

**Remaining fix — two edits, both in the test, neither of which I made** (this sweep is
read-only):
1. Drive the "always dispatchable" loop from `arm_runs_in_this_build` itself, or exclude
   both gated ids, so it cannot rot again on the next arm.
2. Replace the literal `19` with the relation the same test computes eleven lines above
   (`declared.len() - not_runnable + 1`) — feature-setting-agnostic, and the one form that
   survives enabling either feature.

The comment above that literal (*"in the SHIPPED build (no backtest/yahoo) the honest number
is 19"*) names only `yahoo` and is now a two-feature question.

#### 1d. **RESOLVED at 09:51 — the double-subtraction the first fix introduced**

Recorded because it is the most instructive part of this entry: **for ~15 minutes the fix
moved the defect to a different coin population rather than closing it.**

When the build-time drop landed at 09:36, the pre-existing **coin-time** drop was not
revisited, so the DVOL arm was subtracted **twice** for coins outside {BTCUSDT, ETHUSDT} —
and only one such arm exists:

| | declared | dropped by build gate | + buy-and-hold | screen showed | actually runs | verdict |
|---|--:|--:|--:|--:|--:|---|
| BTCUSDT / ETHUSDT | 19 | 2 (macro, dvol) | +1 | **18** | 18 | ✅ fixed at 09:36 |
| SOLUSDT (any other coin) | 19 | 2 (macro, dvol) | +1 | **17** | 18 | ❌ under-counted 09:36→09:51 |

Before 09:36, BTC/ETH over-counted by one and other coins were right. Between 09:36 and
09:51, BTC/ETH were right and every other coin under-counted by one. **[SRC]** The 09:51
edit closes it with the build-gate guard:
```rust
if backtest::bakeoff::dvol_arm_compiled() && !backtest::bakeoff::dvol_supported(symbol_str) {
    full - 1
} else { full }
```

**The lesson generalises past this arm.** The fix changed a predicate that a *second,
independent* subtraction was silently depending on — the two gates being **F-6**, one in
`backtest` and one in `ui`, over the same fact. Whenever an arm can be absent for two
independent reasons, the count must charge each reason at most once; nothing in the type
system enforces that, and the transient defect was invisible to every test because the test
that covers this function (`arm_count_drops_the_dvol_arm_for_unsupported_coins`) asserts the
*relation* `full - 1` rather than a ground truth, so it passed throughout.

#### 1e. Original evidence chain (preserved — this is what the fix rests on)

`arm_runs_in_this_build()` was introduced by the #81 fix so the operator-facing arm count
can never claim an arm the dispatch loop drops. It handles exactly one arm.

**Evidence chain — every link verified:**

1. **[SRC]** `crates/backtest/src/bakeoff/mod.rs:186-191` —
   ```rust
   pub fn arm_runs_in_this_build(strategy_id: &str) -> bool {
       match strategy_id {
           "v0.macro_riskon" => macro_arm_compiled(),
           _ => true,
       }
   }
   ```
   `v0.dvol_regime` therefore returns `true`.
2. **[CARGO]** In the shipped cockpit build `backtest` resolves with **zero features**
   (`cargo tree -p ui -e normal --format '{p}||{f}'` → `backtest v0.1.0 ||`). Also zero
   under `-p agent`, under `-p backtest` itself, and under a workspace-wide
   `-e normal,dev` resolution (the CI shape).
3. **[SRC]** So `resolve_dvol_override` is the `#[cfg(not(feature = "realdata"))]` variant
   at `crates/backtest/src/bakeoff/mod.rs:341-350`, which returns `None`
   **unconditionally** — no corpus check, no I/O, no branch.
4. **[SRC]** `crates/backtest/src/bakeoff/mod.rs:~1204` —
   `if is_dvol_arm && dvol_override.is_none() { … continue; }`. The arm is dropped on
   **every** run of every shipped build.
5. **[SRC]** `crates/ui/src/leaderboard/runner.rs:55-61` — `advisor_field()` = 10 + 8 + 1 =
   **19 declared arms**, `v0.dvol_regime` among them (`bakeoff/mod.rs:684`).
6. **[SRC]** `crates/ui/src/leaderboard/runner.rs:84-90` — `advisor_field_arm_count()`
   filters the 19 through `arm_runs_in_this_build` (drops only macro → 18) and adds 1 for
   buy-and-hold → **19**.
7. **[SRC]** `crates/ui/src/leaderboard/runner.rs:109-116` +
   `crates/ui/src/screens/leaderboard.rs:276-281` — for BTCUSDT `dvol_supported` is `true`,
   so **19 is what renders on the leaderboard**.
8. **[SRC]** The `field` handed to `run_bakeoff` is the *unfiltered* `advisor_field()`
   (`runner.rs:150`, `runner.rs:186`), so the loop receives 19, drops macro **and** dvol,
   and ranks 17 + buy-and-hold = **18**.

**The test written to prevent exactly this encodes the wrong number.**
`crates/ui/src/leaderboard/runner.rs:360-366` hard-asserts
`advisor_field_arm_count() == 19` with the message *"shipped build: 18 runnable arms +
buy-and-hold (the macro arm is ABSENT)"*, and the loop at `runner.rs:346-355` asserts
*"Every other declared arm is always dispatchable"* — which is **false** for
`v0.dvol_regime` in the shipped build.

**And the doc-comment mis-frames it the way bug-log's own 2026-08-15 changelog says #78
mis-framed it.** `runner.rs:104-107` calls DVOL an arm that "can *additionally* be dropped
at run time when their corpus is missing" — a data problem. In the shipped build it is a
**build-time certainty**: the loader is not compiled, so no corpus and no fetch can change
it. Fetching the DVOL parquets cannot move this number.

**Known vs new**: the *drop* is known (bug-log #81 changelog, 2026-08-15, records the
`realdata` twin and that the drop-to-ABSENCE guard already landed). The **count
discrepancy is not recorded anywhere**, and it is the operator-facing half — the same harm
#81 was raised for.

**Fix shape** (one line, plus the test): extend `arm_runs_in_this_build` with
`"v0.dvol_regime" => cfg!(feature = "realdata")`, and change the test's terminal assertion
from the literal `19` to the relation it already computes.

---

### N-2 — CRITICAL. The #80 short-leg bypass is **unfixed in the forward paper loop** — the path that actually executes the operator's plan. Its comment now asserts an invariant that is false.

#80 was fixed in `crates/backtest/src/scenarios/sma_composed_run.rs` (the ranking path).
The agent's forward/paper loop retains the pre-fix shape byte-for-byte in structure.

**[SRC]** `crates/agent/src/runtime.rs:2186-2188`, verbatim:

> *"ADR-0068 D6: for short operations, call `short_exec` helpers directly (same as
> `sma_composed_run.rs` and bake-off) rather than routing through the matching engine
> (which assumes long)."*

**That parenthetical is now false.** **[SRC]** `sma_composed_run.rs:650` calls
`short_exec::plan_open_short` and then **`engine.step(&bar, ord)` at `:677`** — it routes
through the engine, and the engine is built with the venue filter at `:488-489` and
`:1102-1103`. The agent loop does not: `PaperEngine` is constructed at `runtime.rs:2094`
without `.with_venue_filter_mode`, and both short branches end in
`continue; // handled; skip matching-engine path` (`runtime.rs:2226`, `runtime.rs:2263`)
after hand-synthesising a `trading_core::Fill` at `bar.close` charging only
`notional * taker_bps`.

**Reachability confirmed** — this is not dormant code: `runtime.rs:1520` computes
`fwd_short_enabled = backtest::BakeoffConfig::is_short_enabled(...)` and passes it at
`:1541`, so any crowned `_ls` arm promoted to a forward run enters this path.

**Why this is worse than #80.** #80 is a *comparability* defect — it flatters a short arm's
rank. This is the **execution** path: the forward paper run is what the product tells the
operator their €200 is doing. Long legs pay slippage and lot-rounding there; short legs pay
neither.

**Nothing can catch it**: **[SRC]** the #80 parity gate
`crates/backtest/tests/short_long_friction_parity_e2e.rs` drives `run_scenario` inside the
`backtest` crate and cannot observe `crates/agent`.

**Secondary, same site [SRC]**: the accounting halves also diverge — the cover branch does
`realized_pnl += -notional - fee_amount;` (`runtime.rs:2224`) while the open-short branch
never touches `realized_pnl` at all, and neither uses the shared `apply_engine_fill`
(`sma_composed_run.rs:921`) that the long path uses.

---

### N-2b — HIGH, and it **corrects bug-log #82's framing**. The five short arms are in **no shipped advisor field**. The cockpit never ranks them — but it carries full display labels for them.

Bug-log #82 opens *"The advisor's entire SHORT SLATE never shorts on real data. Five arms
are **ranked** as long/short"*. The arms exist, are dispatchable, and take zero short legs —
that half stands. But they are not in the field the product races.

**Evidence — exhaustive, not sampled:**

1. **[SRC]** `crates/ui/src/leaderboard/runner.rs:55-61` — `advisor_field()` composes
   `default_field()` + `default_ensemble_field()` + `default_macro_field()`.
   `default_short_field()` is **not** among them.
2. **[GREP]** `grep -rn 'field: ' crates/ui/src/leaderboard/ crates/backtest/src/bakeoff/` —
   the `BakeoffRequest.field` is written at exactly **two** sites in the whole UI,
   `runner.rs:150` and `runner.rs:186`, both `advisor_field()`.
3. **[GREP]** `grep -rn 'default_short_field' crates/` — the only non-definition reference
   in the workspace is `crates/backtest/tests/p2_verdict_rerun.rs`, a **test**.
4. **[GREP]** `grep -rn 'run_bakeoff(' crates/` — the only production caller is
   `crates/ui/src/leaderboard/runner.rs:260`. Every other caller is a test.
5. **[SRC]** `crates/backtest/src/bakeoff/mod.rs:670-686` — `default_field()` is 10 arms;
   +8 ensembles +1 macro = the 19 of N-1. No short arm among them.

**So #82's per-arm fill counts came from the research harness**
(`crates/backtest/tests/short_bakeoff_bear_bull.rs`, 18 references), not from the cockpit.

**The part that is a defect in its own right — the UI is dressed for arms that never
arrive.** **[SRC]** `crates/ui/src/screens/leaderboard.rs` carries a complete label mapping
for all five (`:1572-1576`), a predicate `is_short_strategy` (`:1610`), and **three test
blocks** asserting those labels render (`:1832-1836`, `:1857-1861`, `:1974-1984`) — with the
comment at `:1827`: *"extended ui-side or the rows show raw `sma_cross_ls` etc."* Those
tests pass by calling the label function directly; no test asserts the arms reach a row,
because they cannot.

**Why it matters, and why it cuts both ways.** The honest reading is *better* than #82
suggests for the product surface: the operator is **not** shown a mislabelled short slate,
because they are shown no short slate at all. But it is worse in a different direction — the
five arms are fully implemented, registered in **both** dispatch tables (`run_scenario` and
`build_registry_for`), carry dedicated e2e tests (`short_bakeoff_bear_bull.rs`,
`short_long_friction_parity_e2e.rs`), motivated the #80 engine-routing fix, and are ranked
by nothing. **#82's fix decision changes depending on this**: "re-label four arms as
long-only" is the wrong move if they are not in the field; "decide whether the short slate
ships at all" is the real question.

---

### N-3 — HIGH. The two **drawdown / daily-loss kill-switch limits** are configured, defaulted, documented as tripping the kill switch, and read by **nothing**.

**[GREP]** `grep -rn 'daily_loss_stop_pct\|max_drawdown_stop_pct' --include="*.rs" --include="*.toml" crates/ config/` — complete output, 15 lines:

- `crates/agent/src/config.rs:509,510` — field declarations
- `crates/agent/src/config.rs:519,520` — defaults (`-5.0` / `-15.0`)
- `crates/agent/src/config.rs:1282,1283` — the doc-TOML inside a doc-comment
- `config/agent.toml:36,37` — the operator's live config
- 8 lines in **4 test files** (`crates/ui/tests/live_forward_pnl_render.rs:398-399`,
  `crates/agent/tests/{equity_store_integration,paced_replay_late_subscriber,reflection_wiring_regression}.rs`)

**Zero production reads.** **[SRC]** `spawn_trading_loop` (`crates/agent/src/runtime.rs:1998`)
takes `risk_cfg: &RiskConfig` and reads exactly two fields —
`risk_cfg.sizing.fixed_fraction` (`:2047`) and `per_symbol_exposure_cap` (`:2054`) — then
builds `RiskLimits { per_symbol_exposure_cap, price_sanity_band, portfolio_exposure_cap }`
(`:2053-2057`), a struct that **has no drawdown or daily-loss field at all**
(`crates/core/src/order.rs:60-71`). `Config::validate()` (`config.rs:990-1055`) does not
even range-check them.

**The claim that is false**: `docs/archive/pre-bmad-spec/v1/paper-soak-longevity/runbook-realtime-soak.md:293-295`
— *"For paper mode, the kill switch trips on: `max_drawdown_stop_pct` exceeded …
`daily_loss_stop_pct` exceeded"*. **[SRC]** `HaltReason`
(`crates/agent/src/kill_switch.rs:48-56`) has no such variant.

This is #79's shape applied to a **safety** control rather than a realism control.

---

### N-4 — HIGH. `HaltReason::HeartbeatTimeout` is unreachable: its monitor is **never spawned**.

**[GREP]** `grep -rn 'spawn_heartbeat_monitor' --include="*.rs" .` returns **exactly one
line** repo-wide — `crates/agent/src/kill_switch.rs:361`, the definition itself. No caller,
not even a test.

Consequently **[GREP]** `KillSwitchConfig.heartbeat_timeout_ms` (`config.rs:666`,
`config/agent.toml:59` = `5000`) has zero readers: all 13 `kill_switch.<field>` production
hits are `halt_file`, none heartbeat. The sibling `halt_file` **is** live
(`runtime.rs:842`, `main.rs:214`, `cockpit_live.rs:315`) — the asymmetry is what makes this
a wiring gap rather than a design choice.

---

### N-5 — HIGH. The clock-skew detector is **never constructed**; both skew thresholds are inert, and the module doc claims the opposite.

**[GREP]** `grep -rn 'ClockSkewDetector' --include="*.rs" crates/` — complete output, 5
lines: the `lib.rs:40` re-export, the struct + impl at `clock_skew.rs:59/64`, and its own
test helper at `clock_skew.rs:145-146`. **Zero production construction sites.**

**[SRC]** `DataConfig.clock_skew_warn_ms` / `clock_skew_halt_ms` (`config.rs:429-431`,
`config/agent.toml:33-34`) are read only by `Config::validate()` comparing them **to each
other** (`config.rs:1045-1051`).

**[SRC]** `crates/data/src/clock_skew.rs:4-5` asserts it "trips the kill switch (returns
`ObserveResult::TripKillSwitch`) when skew exceeds `halt_ms`". Nothing calls `.observe()`
in production.

---

### N-5b — HIGH. The **entire GARCH volatility-forecast capability is unreachable**: a 289-line provider module with zero references, and two strategies reachable only from their own passing e2e tests.

Three independent layers, each verified:

1. **[GREP]** `crates/forecast/src/vol.rs` — **289 lines, zero references outside itself.**
   `grep -rn 'VolForecastProvider\|GarchVolForecaster\|VolRequest\|VolResponse' crates/`
   excluding that file returns **nothing**. Its own header (`vol.rs:14-15`) names the
   intended consumers — `VolTargetingOverlay`, `VolKillSwitchOverlay`,
   `VolMeanReversionStrategy` holding `Arc<dyn VolForecastProvider>`. **That wiring does not
   exist.** This is #81's shape one notch worse: the module *is* compiled, and is still
   unreachable — so no feature flag can be blamed and no build change can fix it.
2. **[GREP]** `crates/strategy/src/lib.rs:119` `with_garch_vol_strategy` and `:157`
   `with_garch_vol_kill_switch` — **zero callers**. Consequently `VolKillSwitchOverlay::new`
   is constructed only in `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs`
   (4 sites) and the crate's own `#[cfg(test)]` block, and `VolMeanReversionStrategy::new`
   only from the dead builder at `lib.rs:124`.
3. **The control that proves it is a gap, not a design choice**: the *third* sibling builder
   **is** wired — **[SRC]** `crates/backtest/src/scenarios/garch_vol_target_overlay.rs:164`
   calls `strategy::with_garch_vol_overlay_momentum`. Two of three shipped; the wiring for
   the other two was never written.
4. **[GREP]** `crates/strategy/src/vol_targeting_overlay.rs:764` `checkpoint_loader` — the
   `mod` declaration is its **only** reference workspace-wide; `load_params` (`:788`) has
   zero callers. Trained GARCH checkpoints are never loaded from disk. Note it is
   **double-suppressed**: `#[cfg(feature = "forecast")]` + `#[allow(dead_code)]` — and the
   feature **is** on (§3), so only the `allow` is hiding it.

This directly engages the project's own AD-16 non-negotiable ("every strategy overlay ships
with a baseline-equity-divergence e2e test from day 1"). Here the e2e tests exist and
**pass**, and the overlay still has no production entry point — the 2026-05-22 no-op-overlay
precedent, one level up the call graph.

---

### N-5c — MEDIUM/HIGH. `PaperEngine::new(config, seed)` — the **seed cannot affect anything**. Its RNG is constructed and never read.

**[SRC]** `crates/backtest/src/paper.rs:69` declares `rng: ChaCha20Rng`; `:85` writes
`rng: ChaCha20Rng::seed_from_u64(seed)`. **[GREP]** `grep -n '\.rng' crates/backtest/src/paper.rs`
returns **nothing** — there is no field access to `rng` anywhere in the file. The `seed`
parameter is threaded from every construction site and is provably inert for this engine.

**Why it is worth naming rather than shrugging at.** The bake-off's apples-to-apples
guarantee is stated as *"same-seed-every-arm"* (`crates/ui/src/leaderboard/runner.rs:140`,
enforced inside `run_bakeoff`). That guarantee is real where the seed reaches a live RNG
(the bootstrap gate, strategy RNGs); it is **decorative** at the matching engine. Anyone
reading `PaperEngine::new(cfg, seed)` will reasonably conclude execution is seeded and
reproducible-by-seed, and will conclude wrongly. The in-source comment at `paper.rs:68`
("T24 will use it more extensively") dates the intent; T24 did not land.

**[UNVERIFIED]**: whether any *other* engine (`crates/exec/src/paper.rs`) reads its seed. Not
checked.

---

### N-5d — MEDIUM. `MomentumStrategy::drift_threshold` is **validated, then discarded** — the drift-rebalance knob never fires.

**[GREP]** `grep -rn 'drift_threshold' crates/strategy/src/` — complete output, 5 lines:
the field declaration (`cross_sectional/momentum.rs:39`), one write
(`momentum.rs:194`, `drift_threshold: cfg.drift_rebalance_threshold`), and three lines of
**error-code plumbing** (`cross_sectional/config.rs:141` — *"drift_rebalance_threshold must
be in (0, 1)"*, `:221`, `:724`). **Zero reads.**

The shape is worth naming precisely: the config has a *rejecting validator* for a value
nothing consumes. A validator is the strongest possible signal to a reader that a field is
live — it is the one thing a purely decorative field usually lacks. Add
`crates/strategy/tests/bad_v1_strategy_fixtures.rs:80`, which asserts the rejection, and
three independent artefacts attest to a knob that does nothing.

---

### N-5e — MEDIUM. The Trail drawer's **Signal and Forecast panels can never render**, and the four-table audit join behind them has only test callers.

**[GREP]** `grep -rn 'DrawerPayload::' crates/ui/src/` excluding the widget itself — the
**only** producers are `crates/ui/src/screens/trail.rs:54` and
`crates/ui/src/gallery/routes.rs:850`, both constructing `DrawerPayload::Fill`. The
`Signal` arm (`widgets/trail_drawer.rs:141`) and `Forecast` arm (`:172`) are unreachable
match arms.

**The cause is one un-landed hop, self-documented.** **[SRC]**
`crates/reflection/src/trail_mirror.rs:306`, verbatim:
`// T-D-N25 replaces this body with the real audit::query::trail_for_fill_id call.`
It was not replaced. **[GREP]** `trail_for_fill_id` (`crates/audit/src/query.rs:2272`) has
callers **only** in `crates/audit/tests/trail_reconstruction.rs` (3 sites). The UI hydrates
from the stub at `crates/ui/src/live.rs:673`.

Downstream, four public types are unreachable from production: `TrailFillRow`
(`query.rs:2190`), `TrailSignalRow` (`:2205`), `TrailForecastRow` (`:2226`),
`TrailReconstruction` (`:2246`). Acknowledged in-tree at `crates/ui/src/screens/trail.rs:231`
(*"NOTE (data-plumbing follow-up)"*) — so this is a **labelled** gap, which is why it is
MEDIUM and not HIGH.

---

### N-6 — HIGH. **5,310 lines of `crates/backtest/src/` are compiled into zero shipping builds and zero CI runs**, and three test targets are silently skipped in CI. This generalises #81's twin from "two features" to a measured blast radius.

**[CARGO]** `backtest` resolves to **no features** in every entry point measured:

| resolution | command run | `backtest` features |
|---|---|---|
| cockpit / cockpit\_live / viewer | `cargo tree -p ui -e normal` | *(none)* |
| documented cockpit\_live cmd | `cargo tree -p ui --features live -e normal` | *(none)* |
| gallery-only | `cargo tree -p ui --no-default-features --features fixtures -e normal` | *(none)* |
| headless agent (`trading` bin) | `cargo tree -p agent -e normal` | *(none)* |
| backtest's own default bins | `cargo tree -p backtest -e normal` | *(none)* |
| **CI** (`cargo test --workspace`) | `cargo tree --workspace -e normal,dev` | *(none)* |

**[GREP]** `grep -rn 'backtest/' --include=Cargo.toml .` → **zero hits**. Nothing in the
workspace enables `backtest/yahoo`, `backtest/realdata`, or `backtest/candle`.

**[SRC]** What that gates out (`crates/backtest/src/lib.rs:71-105`, line counts by `wc -l`):

| module | lines | gate |
|---|---|---|
| `dvol_data.rs` | 1,491 | `realdata` |
| `basis_data.rs` | 1,464 | `realdata` |
| `funding_data.rs` | 1,093 | `realdata` |
| `macro_regime.rs` | 935 | `yahoo` (`#![cfg]` floor-gate) |
| `realdata.rs` | 327 | `realdata` |
| **total** | **5,310** | |

Plus the `realdata`/`candle` `#[cfg]` blocks scattered through `main.rs` (~20 sites),
`bin/param_robustness_sweep.rs` (~10), `bin/monte_carlo.rs`, `bakeoff/mod.rs`, and three
`scenarios/*.rs`.

**The CI consequence is the part worth naming.** Because CI resolves `backtest` with no
features, `cargo test --workspace` **cannot compile these 5,310 lines at all** — a syntax
error in them would not fail CI. And **[CARGO]** (`cargo metadata`) three test targets are
*silently skipped*, not reported:

| target | needs | runs in CI? |
|---|---|---|
| `backtest::p2_verdict_rerun` | `realdata,yahoo` | **no** |
| `backtest::threshold_sweep_readonly` | `candle,realdata` | **no** |
| `backtest::run_yahoo_sma_ticker_flag` | `yahoo` | **no** |

(The nine `forecast` targets and one `strategy` target gated on `candle`/`forecast` **do**
run — those features are enabled transitively. See §3.)

`p2_verdict_rerun` is the multi-corpus harness behind the era-qualified thesis. It has
never executed in CI.

---

### N-7 — MEDIUM. `backtest/candle` is a **third** instance of the enabled-nowhere shape. The bug-log's detector table listed seven features; the workspace declares **24**.

**[CARGO]** `cargo metadata` gives the authoritative set — 24 features across 6 crates, not
7. `backtest/candle` was absent from bug-log #81's table entirely.

**[GREP]** Nothing enables it (same zero-hit grep as N-6). **[SRC]** It gates
`scenarios/tcn_overlay_weights.rs:35`, `scenarios/patchtst_overlay_weights.rs:50`, and
`scenarios/threshold_sweep.rs:63`.

**Severity is genuinely lower than #81, and the reason is the lesson.** All three
`#[cfg(not(feature = "candle"))]` arms **`anyhow::bail!` with the exact rebuild command**
(e.g. `tcn_overlay_weights.rs:36-41`). They degrade to a **loud error**, never to a silent
stub. That is the correct shape, and it is why this is MEDIUM while #81 was CRITICAL —
worth recording as the positive control for what the fix to #81-class defects looks like.

Practical status: those three scenarios are **unreachable from every shipped build** and
reachable only from the documented `cargo run -p backtest --features realdata,candle`
(README:130, README:175). They are not in `advisor_field()`, so no operator surface counts
them.

---

### N-8 — MEDIUM. `AuditEvent::ForecastEmitted` is **never emitted in any build of the workspace** — a three-crate feature chain that nothing enables.

**[CARGO]** The chain is `agent/forecast-audit-tick` → `strategy/forecast-audit-tick` →
`forecast/audit-tick`. **[GREP]** `grep -rn 'audit-tick' --include=Cargo.toml crates/`
returns only the three declarations — **no enabler**, no `default`, and `ui`'s feature list
does not mention it. Confirmed off in every resolution in the N-6 table.

**[SRC]** The only two producers of the variant are
`crates/forecast/src/tcn.rs:860` and `:994`, both inside `#[cfg(feature = "audit-tick")]`
blocks. **[GREP]** There are no consumers or tests of `ForecastEmitted` either — the tests
in `crates/trader/tests/llm_forecaster_audit_tick.rs` all target `LlmForecastEmitted`, a
different variant.

So the audit ledger has a declared event type with zero live producers in every build. This
was not in the bug-log's table.

---

### N-9 — MEDIUM. The R9 **strategy-decay** subsystem is disconnected at three independent layers, any one of which alone would make it inert.

**[GREP]** `grep -rn 'sharpe_ratios(\|decay_fired(\|SharpeFn' crates/reports crates/agent crates/ui crates/trader` — complete output:

1. **No caller.** `reports::render::risk_metrics::sharpe_ratios` (`risk_metrics.rs:294`) has
   **zero callers workspace-wide** — only its own definition and doc-comments.
2. **It could not be wired even if someone tried.** **[SRC]** The consumer type is
   `pub type SharpeFn = fn(&[Decimal]) -> SharpeStats` (`risk_metrics.rs:31`), but
   `sharpe_ratios` is `fn(&[Decimal], u32) -> SharpeStats`. The signatures do not match —
   a compile-level proof the two halves were never connected.
3. **And it could never fire.** **[SRC]** `risk_metrics.rs:294-299`:
   ```rust
   pub fn sharpe_ratios(equity: &[Decimal], cadence_minutes: u32) -> SharpeStats {
       SharpeStats {
           inception: sharpe(equity, cadence_minutes),
           last_7d:   sharpe(equity, cadence_minutes),   // ← identical call
       }
   }
   ```
   `inception` and `last_7d` are the **same call**. The decay heuristic compares them, so it
   is structurally incapable of detecting decay.

**[SRC]** `decay_fired` / `decayed_strategies` (`memory_highlights.rs:202`, `:216`) are
called only from `mod tests` at `:315`/`:325`, with a `synthetic_sharpe` fixture — i.e. the
tests pass because they supply the only implementation that works.

---

### N-10 — MEDIUM. `LatencySlippageSimConfig.latency_ms_min` / `latency_ms_max` are #79's direct siblings and are **INERT**: `exec::apply_latency` has zero production callers.

**[GREP]** `grep -rn 'apply_latency' --include="*.rs" .` — complete output is the definition
(`crates/exec/src/latency.rs:58`), the `lib.rs:13` re-export, **two criterion benches**
(`benches/latency_slippage.rs`, `benches/throughput_with_sim.rs`), and its own `mod tests`.
No production call site anywhere.

**[SRC]** The two fields are threaded from CLI flags (`crates/backtest/src/main.rs:112,116`)
into `ScenarioConfig.latency_slippage_sim` at ~15 sites in `main.rs`, then into every
scenario-input struct — and never read for behaviour. The only in-crate read is
`is_noop()` (`cli_types.rs:152-153`), whose sole caller is a test.

Note the **benches are the trap**: a benchmark exercising a function is easy to mistake for
a production caller. Contrast the siblings `slippage_model` and `volume_usd_per_symbol`,
both genuinely LIVE at `crates/backtest/src/scenarios/sim.rs:63` and `:79`.

---

### N-11 — LOW/MEDIUM. `MatchConfig.maker_fee_bps` is configured from `config/agent.toml` and is structurally unreachable — every fill hardcodes Taker.

**[SRC]** Written: `config/agent.toml:44` → `BacktestConfig.maker_fee_bps` (`config.rs:530`)
→ `runtime.rs:2039` → `MatchConfig`; plus 12 hardcoded `maker_fee_bps: 2` sites across
`crates/backtest/src/scenarios/*` and `engine.rs:2622`.

**[SRC]** Read: `PaperEngine::step` reads exactly three config fields —
`fill_price_mode` (`paper.rs:123`), `slippage_bps` (`:128`), `taker_fee_bps` (`:130`).
Every fill sets `fee_tier: FeeTier::Taker` (`paper.rs:200`) and
`liquidity: Liquidity::Taker` (`:203`), so no maker branch exists to reach.
The fourth read, `self.config.clone()` at `paper.rs:213`, is the
`MatchingEngine::config()` getter — **[GREP]** which has zero callers.

---

### N-12 — LOW. A documented operator entry point **does not build**.

**[CARGO]** Actually run:
```
$ cargo build -p ui --bin cockpit
error: target `cockpit` in package `ui` requires the features: `fixtures`
```
**[SRC]** `docs/runbooks/advisor-end-to-end-demo.md:259` documents exactly
`cargo run -p ui --bin cockpit` with no `--features`. `ui`'s default is
`["live","yahoo","binance"]`, which does not include `fixtures`, and the bin declares
`required-features = ["fixtures"]` (`crates/ui/Cargo.toml:17-20`).

The correct form (`--features fixtures`) is used in CI (`.github/workflows/ci.yml:106`) and
in `docs/runbooks/cockpit-cross-platform.md:80`. The end-to-end demo runbook — the
operator-facing one — is the copy that is wrong.

---

### Lower-tier config-inertness findings (verified, listed for completeness)

Each verified by exhaustive grep; see §5 for the write/read split.

| # | field | status |
|---|---|---|
| N-13 | `AuditConfig.reconciliation_tolerance_usdt` | INERT — never assigned to `ReconcilerState.tolerance`; `ReconcilerState` is constructed only in tests |
| N-14 | `BinanceSourceConfig.rest_url` | INERT — all 4 `BinanceFeed::new` sites pass **ws\_url twice** (`runtime.rs:1127`, `:1303`, `:1503`, `:1561`); Coinbase + Kraken siblings do read theirs |
| N-15 | `SignalLogConfig.enabled` | INERT — zero production reads; three other crates' comments describe it as a live gate |
| N-16 | `SmaCrossoverConfig.enabled` | INERT — the strategy is registered unconditionally (`runtime.rs:208`, `:247`, `:361`); setting `false` does nothing. Contrast `tcn_overlay_momentum.enabled`, which **is** honoured (`runtime.rs:260`) |
| N-17 | `TcnOverlayMomentumConfig.base_config_path`, `PatchTstOverlayMomentumConfig.base_config_path` | INERT — 4 grep hits total, all declaration/default; zero field accesses |

---

## 2. Entry-point table

Every way this system can execute. `required-features` **gates a binary but propagates
nothing to library consumers** — the trap that hid #81. **[CARGO]** all rows from
`cargo metadata --no-deps`; feature columns from `cargo tree -e normal`.

### 2.1 Binaries

| crate | bin | `required-features` | ships in operator flow? | its own crate's resolved features |
|---|---|---|---|---|
| `ui` | **`cockpit_live`** | `live` | **yes — the product** | `binance,default,live,yahoo` |
| `ui` | `cockpit` | `fixtures` | demo/CI only | `binance,default,fixtures,live,yahoo` |
| `ui` | `ui-gallery` | `fixtures` | dev only | as above |
| `ui` | `viewer` | — | yes (offline report viewer) | `binance,default,live,yahoo` |
| `agent` | **`trading`** | — | yes (headless agent) | *(none)* |
| `backtest` | `backtest` | — | yes (CLI) | *(none)* unless `--features` passed |
| `backtest` | `monte_carlo` | — | research | *(none)* unless passed |
| `backtest` | `param_robustness_sweep` | — | research | *(none)* unless passed |
| `backtest` | `threshold_sweep` | `candle,realdata` | research | those two, when built |
| `backtest` | `run_yahoo_sma` | `yahoo` | research | `yahoo` |
| `data` | `fetch_binance_klines` / `_funding` / `_premium` | — | corpus fetch | `yahoo,yahoo-online` (via workspace) |
| `data` | `fetch_coinbase_klines`, `fetch_deribit_dvol` | — | corpus fetch | as above |
| `data` | `fetch_yahoo_klines` | `yahoo-online` | corpus fetch | `yahoo,yahoo-online` |
| `forecast` | `train_garch`, `vol_verdict`, `regime_verdict`, `sharpe_comparison` | — | research | `candle,default` |
| `forecast` | `train_tcn`, `train_patchtst`, `forecast_distribution`, `recalibrate_sigma_train` | `candle` | research | `candle,default` |
| `reports` | `report` | — | operator reports | *(none)* |
| `trader` | `llm_verdict` | — | research | *(none)* |
| `llm` | `llm-smoke`, `generate-replay-fixture` | — | dev/smoke | *(none)* |

Examples: `backtest::passive_baseline_equity` (`realdata`), `data::{basis,stablecoin,universe}_diag` (—).

### 2.2 Documented run commands, and what they actually build

**[SRC]** from `README.md`, `docs/runbooks/*.md`, `.github/workflows/ci.yml`.

| command | source | `backtest` features it produces |
|---|---|---|
| `cargo run -p ui --release --bin cockpit_live --features live` | README:151 | **none** |
| `cargo build --release -p ui --bin cockpit_live --features live` | README:127 | **none** |
| `cargo run -p ui --bin cockpit` | runbook advisor-e2e:259 | **fails to build** — see N-12 |
| `cargo build -p ui --bin cockpit --features fixtures` | ci.yml:106, runbook cross-platform:80 | **none** |
| `cargo run --bin trading -- --config config/agent.toml` | runbooks | **none** |
| `cargo run -p backtest --release -- --scenario … --seed …` | README:172 | **none** |
| `cargo run -p backtest --release --features realdata,candle --bin backtest -- …` | README:175 | `realdata,candle` |
| `cargo build --release -p backtest --features realdata,candle` | README:130 | `realdata,candle` |
| `cargo run -p backtest --features realdata --example passive_baseline_equity` | runbook | `realdata` |
| `cargo test --workspace --exclude ui` | ci.yml:115 | **none** |
| `cargo test -p ui --features fixtures` (Linux/Windows) | ci.yml:124,153 | **none** |
| `cargo test -p ui` (macOS canonical gate) | ci.yml:144 | **none** |

**Note the shape**: `--features live` on the `ui` invocation is a no-op — `live` is already
in `ui`'s `default`. And **no documented command that an operator runs for the product
passes anything to `backtest`.** The only commands that do are the research CLI ones.

### 2.3 The `required-features` trap, stated plainly

**[SRC]** `crates/backtest/Cargo.toml` puts `required-features = ["realdata"]` on
`threshold_sweep` and on the `passive_baseline_equity` example, and
`required-features = ["yahoo"]` on `run_yahoo_sma`. Reading the manifest, `realdata` and
`yahoo` look "used". They gate **those binary targets only**. `ui` and `agent` depend on
the `backtest` **library**, which those declarations do not touch. This is exactly why
manifest reading was insufficient and every row above came from the resolver.

---

## 3. The feature-propagation matrix

**24 declared features across 6 crates** **[CARGO]** (`cargo metadata`). The bug-log's #81
table listed 7; the missing 17 are where N-7 and N-8 were hiding.

Legend: ✅ on · ❌ off · *(n/a)* feature does not exist for that crate.
Columns: **Cockpit** = `ui` default (cockpit\_live / cockpit / viewer) · **Agent** =
`-p agent` (`trading` bin) · **BT-def** = `-p backtest` default bins · **BT-doc** = the
documented `--features realdata,candle` CLI build · **CI** = `cargo test --workspace`.

| capability (what it gates) | feature | Cockpit | Agent | BT-def | BT-doc | CI | method |
|---|---|:--:|:--:|:--:|:--:|:--:|---|
| Macro-regime loader (`macro_regime.rs`, 935 ln) — `v0.macro_riskon` | `backtest/yahoo` | ❌ | ❌ | ❌ | ❌ | ❌ | [CARGO] |
| DVOL / basis / funding loaders + real `resolve_dvol_override` (4,375 ln) | `backtest/realdata` | ❌ | ❌ | ❌ | ✅ | ❌ | [CARGO] |
| TCN / PatchTST overlay-weights + threshold-sweep cells | `backtest/candle` | ❌ | ❌ | ❌ | ✅ | ❌ | [CARGO] |
| Yahoo parquet **reader** (`data/yahoo.rs`) | `data/yahoo` | ✅ | ✅ | ❌ | ❌ | ✅ | [CARGO] |
| Yahoo **online fetch** (`fetch_and_cache`) | `data/yahoo-online` | ✅ | ❌ | ❌ | ❌ | ✅ | [CARGO] |
| `MockFeed` + test harness scaffolding | `data/fixtures` | ❌ | ❌ | ❌ | ❌ | ✅ (dev-dep) | [CARGO] |
| candle TCN/PatchTST inference (`FeatureTensor = Tensor`) | `forecast/candle` | ✅ | ✅ | ✅ | ✅ | ✅ | [CARGO] |
| candle Metal (Apple GPU) backend | `forecast/metal` | ❌ | ❌ | ❌ | ❌ | ❌ | [CARGO] |
| **`AuditEvent::ForecastEmitted` tee** (tcn.rs:860/994) | `forecast/audit-tick` | ❌ | ❌ | ❌ | ❌ | ❌ | [CARGO] |
| `TcnSyncForecaster` / candle strategy surface | `strategy/forecast` | ✅ | ✅ | ✅ | ✅ | ✅ | [CARGO] |
| propagates the audit-tick tee | `strategy/forecast-audit-tick` | ❌ | ❌ | ❌ | ❌ | ❌ | [CARGO] |
| propagates the audit-tick tee | `agent/forecast-audit-tick` | ❌ | ❌ | *(n/a)* | *(n/a)* | ❌ | [CARGO] |
| in-process cron report scheduler (`agent/src/cron.rs`) | `agent/in_process_cron` | ❌ | ❌ | *(n/a)* | *(n/a)* | ❌ | [CARGO] |
| `Ledger::with_pid` test helpers | `audit/test-support` | ❌ | ❌ | ❌ | ❌ | ❌ | [CARGO] |
| live broadcast bus + `cockpit_live` deps (`ui/live.rs`) | `ui/live` | ✅ | *(n/a)* | *(n/a)* | *(n/a)* | ✅ | [CARGO] |
| Yahoo Lab source chip | `ui/yahoo` | ✅ | *(n/a)* | *(n/a)* | *(n/a)* | ✅ | [CARGO] |
| Binance Lab source chip | `ui/binance` | ✅ | *(n/a)* | *(n/a)* | *(n/a)* | ✅ | [CARGO] |
| deterministic fixture generators for `cockpit`/gallery | `ui/fixtures` | ❌ (default) | *(n/a)* | *(n/a)* | *(n/a)* | ✅ | [CARGO] |
| zero-dim-Quad debug renderer + trace spans | `ui/render-debug` | ❌ | *(n/a)* | *(n/a)* | *(n/a)* | ❌ | [CARGO] |
| geometry-build timer | `ui/chart-build-probe` | ❌ | *(n/a)* | *(n/a)* | *(n/a)* | ❌ | [CARGO] |
| `iced_tester` recorder overlay | `ui/record-tests` | ❌ | *(n/a)* | *(n/a)* | *(n/a)* | ❌ | [CARGO] |
| pass-through to agent cron | `ui/in_process_cron` | ❌ | *(n/a)* | *(n/a)* | *(n/a)* | ❌ | [CARGO] |

### 3.1 Capabilities enabled **NOWHERE that ships** — the flagged set

| feature | flagged? | assessment |
|---|---|---|
| `backtest/yahoo` | 🚩 | bug-log #81. Silent degradation (`None`); honesty half fixed, capability half deliberately blocked pending the emission-cadence fix |
| `backtest/realdata` | 🚩 | bug-log #81's twin. **Silent** — `resolve_dvol_override` returns `None` unconditionally. Drives **N-1** |
| `backtest/candle` | 🚩 **NEW** | **N-7**. Degrades **loudly** (`bail!` with the rebuild command) — the correct shape |
| `forecast/audit-tick` + the two propagators | 🚩 **NEW** | **N-8**. Silent — the tee is simply absent; the audit event has no live producer |
| `forecast/metal` | — | by design (Apple-Silicon opt-in, documented) |
| `agent/in_process_cron`, `ui/in_process_cron` | — | by design (documented opt-in; `ui`'s pass-through is correctly wired) |
| `audit/test-support`, `data/fixtures` | — | test-only by design; **[CARGO]** confirmed absent from every `-e normal` build, so `data/Cargo.toml`'s claim that production excludes `fixtures` is TRUE |
| `ui/render-debug`, `ui/chart-build-probe`, `ui/record-tests` | — | dev/debug opt-in by design |

### 3.2 The mechanism, restated

**Cargo features are per-crate and are not unified across crates.** `ui/yahoo` enables
`data/yahoo` — a *different crate's* feature. It does not touch `backtest/yahoo`. The
workspace therefore *looks* like it enables yahoo everywhere and does not enable it where
the loader lives. The `#![cfg(feature = "…")]` module floor-gate makes the absence
**silent**: the crate compiles, the caller takes the `cfg(not(...))` branch, and the
capability simply is not there.

**The detector, generalised** — for every `cfg(feature)`-gated capability ask two questions,
in this order:
1. *Does any shipping build enable it?* → use `cargo tree -e normal`, never the manifest.
2. *If not, does the `cfg(not(...))` arm **bail** or return a plausible value?* A `bail!`
   (N-7) is a documented limitation. A bare `None` (`backtest/realdata`) or an empty series
   (`backtest/yahoo`) is a defect, because a caller cannot tell it from a real answer.

---

## 4. Dead / unreachable production code inventory

Scope: `crates/*/src/`. "Test-only reachable" is a finding, not an exclusion.

**Method note.** `cargo check --workspace` returned **zero** dead-code warnings (N-0), so
rustc contributed nothing here. Everything below is grep/CodeGraph-verified by the
zero-caller scan: `grep -rn '<name>' crates/*/src/` excluding the definition = 0, and 0 in
`crates/*/tests/` + `crates/*/benches/`.

### 4.1 Unreachable capability — a shipped feature does not work

| item | location | status |
|---|---|---|
| `backtest::{macro_regime, dvol_data, basis_data, funding_data, realdata}` | `crates/backtest/src/` | **NEVER COMPILED** in any shipping or CI build — 5,310 lines. [CARGO] N-6 |
| `forecast::vol` (whole module, 289 ln) | `crates/forecast/src/vol.rs` | **ZERO references workspace-wide.** `VolForecastProvider`, `GarchVolForecaster`, `VolRequest`, `VolResponse` all unreachable. N-5b |
| `strategy::with_garch_vol_strategy`, `with_garch_vol_kill_switch` | `crates/strategy/src/lib.rs:119`, `:157` | **ZERO callers** → `VolKillSwitchOverlay` + `VolMeanReversionStrategy` are **test-only reachable**. N-5b |
| `checkpoint_loader::load_params` | `crates/strategy/src/vol_targeting_overlay.rs:788` | **ZERO callers**; the `mod` decl at `:764` is its only reference. Double-suppressed (`cfg(feature)` + `allow`). N-5b |
| `KillSwitch::spawn_heartbeat_monitor` | `crates/agent/src/kill_switch.rs:361` | **DEAD** — 1 grep hit repo-wide (its own definition) → `HaltReason::HeartbeatTimeout` unreachable. N-4 |
| `kill_switch::remove_halt_file` | `crates/agent/src/kill_switch.rs:399` | **DEAD** — 1 hit. Documented as the "recovery step"; no programmatic recovery path exists |
| `data::ClockSkewDetector` (+ `.observe`, `ObserveResult::TripKillSwitch`) | `crates/data/src/clock_skew.rs:59` | **TEST-ONLY** — 5 hits total; constructed only by its own `mod tests` helper at `:145`. N-5 |
| `audit::query::trail_for_fill_id` + `TrailFillRow`/`TrailSignalRow`/`TrailForecastRow`/`TrailReconstruction` | `crates/audit/src/query.rs:2272`, `:2190`, `:2205`, `:2226`, `:2246` | **TEST-ONLY** — callers only in `crates/audit/tests/trail_reconstruction.rs`. N-5e |
| `DrawerPayload::Signal`, `DrawerPayload::Forecast` | `crates/ui/src/widgets/trail_drawer.rs:141`, `:172` | **NEVER CONSTRUCTED** — the only producers build `::Fill`. N-5e |
| `agent::cron` (whole module) | `crates/agent/src/cron.rs:18` (`#![cfg(feature = "in_process_cron")]`) | **NEVER COMPILED** — feature enabled by no manifest, CI job, or script. Weekly operator success reports never ship. §3 |
| `exec::apply_latency` | `crates/exec/src/latency.rs:58` | **NO PRODUCTION CALLER** — 2 benches + own tests only. N-10 |
| `reports::…::sharpe_ratios`; `decay_fired`, `decayed_strategies` | `risk_metrics.rs:294`; `memory_highlights.rs:202,216` | **DEAD / TEST-ONLY**, and signature-incompatible with `SharpeFn`. N-9 |
| `ReconcilerState`, `ReconcilerTask::new` | `crates/agent/src/reconciler.rs:41` | **TEST-ONLY** — both construction sites are tests; production uses only the free fn `build_snapshot_row` (`runtime.rs:2551`) |
| `VolTargetKind::RealizedVol` | `crates/forecast/src/features.rs:121` | **NEVER CONSTRUCTED** — every construction is `Parkinson`. Its arm at `:800` is `unimplemented!()` — a dead variant fronting a latent panic |
| `backtest::BakeoffConfig::default_short_field()` | `crates/backtest/src/bakeoff/mod.rs:701` | **TEST-ONLY CALLER** — not in `advisor_field()`. N-2b |

### 4.2 Fields written and never read

| field | location | note |
|---|---|---|
| `RiskConfig.daily_loss_stop_pct`, `.max_drawdown_stop_pct` | `crates/agent/src/config.rs:509-510` | N-3 |
| `KillSwitchConfig.heartbeat_timeout_ms` | `config.rs:666` | N-4 |
| `DataConfig.clock_skew_warn_ms`, `_halt_ms` | `config.rs:429-431` | N-5 |
| `LatencySlippageSimConfig.latency_ms_min`, `_max` | `crates/backtest/src/cli_types.rs` | N-10 |
| `MatchConfig.maker_fee_bps` | `crates/backtest/src/paper.rs:30` | fills hardcode `FeeTier::Taker` (`:200`). N-11 |
| **`PaperEngine.rng`** | `crates/backtest/src/paper.rs:69` | written `:85`, **zero `.rng` accesses** → the `seed` parameter is inert. N-5c |
| `MomentumStrategy.drift_threshold` | `crates/strategy/src/cross_sectional/momentum.rs:39` | validated then discarded. N-5d |
| `VolTargetingOverlay.return_vol_rho` | `crates/strategy/src/vol_targeting_overlay.rs:598` | the computed ρ(returns, σ̂) the module doc advertises has no reader |
| `AuditContext.posted_at` | `crates/audit/src/tick.rs:44` | stamped on every tick (`:175`), zero reads |
| `UptimeInterval.boot_id` | `crates/audit/src/query.rs:1528` | written `:1561`, zero reads in any `src/` |
| `ChartProgram.tooltip` | `crates/ui/src/widgets/chart.rs:371` | write-only at `:337`, `:1519`, `:2072`. **Self-documented** at `:357-369` as never read but retained for signature stability — so callers thread a public `tooltip` argument (`chart.rs:326`) with zero effect |
| `TcnOverlayMomentumConfig`, `PatchTstOverlayMomentumConfig` | `tcn_overlay_momentum.rs:51`, `patchtst_overlay_momentum.rs:55` | **never constructed anywhere** — strategies are built via `new()`/`with_passthrough()` with inline args. Subsumes N-17's `base_config_path` |
| `ScenarioConfig.params: Option<ParamSheet>` | `crates/backtest/src/engine.rs:211` | **INERT by declared design** — `ParamSheet` is a unit struct (`:107`). No value can be lost; not a defect |
| serde-shape padding (benign) | `data/src/coinbase.rs:75,100`; `data/src/kraken.rs:80,91`; `forecast/src/features.rs:273` | deserialization padding — not findings |

### 4.3 Zero-caller public items (no capability claim attached)

**[GREP]** each verified by the zero-caller scan:

`EventBus::publish_funding_obs` (`agent/src/bus.rs:190`) · `pair_state::record_staleness_drop`
(`strategy/src/pairs/pair_state.rs:337`) · `audit::tick::into_iter_blocking`
(`audit/src/tick.rs:264`) · `Ledger::with_pid` (`audit/src/ledger.rs:114` — gated on
`audit/test-support`, whose stated purpose is downstream integration tests; **zero test
callers**, so the feature has no consumer at all) · `MatchingEngine::config()`
(`backtest/src/engine.rs:64`) · `PaperEnginePublisher::with_reflection_writer` /
`on_trade_close` (`exec/src/paper.rs:94`, `:123` — a dead **parallel seam**; the capability
is not lost, `agent/src/runtime.rs:2466-2471` enqueues directly) · four TCN BS-2 builders
(`strategy/src/tcn_overlay_momentum.rs:529,574,592,613`) · `ensemble::{member_stances,
last_stance}` (`:189`, `:195`) · `composed/node::{source_path, last_rule_value}` (`:1346`,
`:1361`) · `drawdown_control_overlay::inner_mut` (`:266`) ·
`scenarios/sma_composed::notes_fragment` (`:74`) · `patchtst::random_init_with_seed`
(`forecast/src/patchtst.rs:647`) · `widgets/chart::strategy_label_or_none` (`:1247`) ·
`widgets/positions::warn_if_over` (`:194`) · `strategy::patchtst_sync` (whole 20-line
re-export module; 1 reference workspace-wide — its own `pub mod`).

**Test-only reachable**: `widgets/axis::y_for_value` (`:95`) ·
`widgets/chart_legend::LEGEND_CARD_RADIUS_PX` (`:443` — twin consts, only the `#[cfg(test)]`
one at `:440` is read).

### 4.4 Honestly-labelled dead code (recorded, not defects)

- **[SRC]** `crates/ui/src/screens/strategies.rs` — 371 lines, `#![allow(dead_code)]` at `:9`,
  header states the shell router no longer reaches it and *"Phase D prunes this file"*.
  Confirmed: `shell.rs:195` routes `Screen::Strategies => strategy_registry::view`, and
  `home.rs:17` imports `crate::widgets::{… strategies}` — a **different** module.
- **[SRC]** `crates/reflection/src/audit_tick_consumer.rs:25` `store` — write-only, but the
  module header declares it an observation-only v0.1.0 stub and it is correctly spawned at
  `crates/agent/src/main.rs:172`.

### 4.5 Stale `#[allow(dead_code)]` — six suppressions covering items that are alive

Each **[GREP]**-verified as a false positive; listed so a future sweep does not chase them,
and because a stale suppression is itself a place the detector is off for no reason:

`ui/src/lab/state.rs:256 run_cancel` + `:267 training_cancel` (RAII — `impl Drop` at
`backtest/src/cancel.rs:46` and `ui/src/lab/trainer.rs:65`; the `= None` assignments in
`cockpit_live.rs` **are** the effect) · `ui/src/lab/state.rs:175 training_inflight` (read at
`cockpit_live.rs:1752`, `screens/lab.rs:897`) · `agent/src/runtime.rs:120,125,178`
(destructured at `:818-830`) · `strategy/src/pairs/config.rs:157 stage` (read at `:316`) ·
`ui/src/bin/cockpit_live.rs:1104 kill_switch` (read at `:1318`) ·
`ui/src/widgets/axis.rs:111 x_for_index` (read at `chart.rs:599,1061`) ·
`backtest/src/bin/param_robustness_sweep.rs:384 funding_harvested` (read at `:1873`).

---

## 5. Config-to-consumer chains

For each field: where it is **written** → where it is **read for behaviour** (an `if`, an
argument, an arithmetic operand). Reads that are only `Debug`/`Clone`/`Serialize` derives,
tests, or copies into another config struct do **not** count.

### 5.1 INERT — written, never acted on

| field | written | read | verdict |
|---|---|---|---|
| `RiskConfig.daily_loss_stop_pct` | `config/agent.toml:36`, `config.rs:509/519` | **nowhere** (`RiskLimits` has no such field, `core/order.rs:60-71`) | **INERT** — N-3 |
| `RiskConfig.max_drawdown_stop_pct` | `config/agent.toml:37`, `config.rs:510/520` | **nowhere** | **INERT** — N-3 |
| `KillSwitchConfig.heartbeat_timeout_ms` | `config/agent.toml:59`, `config.rs:666` | **nowhere** (consumer never spawned) | **INERT** — N-4 |
| `DataConfig.clock_skew_warn_ms` / `_halt_ms` | `config/agent.toml:33-34` | only `validate()` comparing them to each other (`config.rs:1045-1051`) | **INERT** — N-5 |
| `LatencySlippageSimConfig.latency_ms_min` / `_max` | CLI `main.rs:112,116` → ~15 threading sites | **nowhere** (`apply_latency` has no production caller) | **INERT** — N-10 |
| `MatchConfig.maker_fee_bps` | `agent.toml:44` → `runtime.rs:2039`; 12 hardcoded sites | **nowhere** (Taker hardcoded) | **INERT** — N-11 |
| `AuditConfig.reconciliation_tolerance_usdt` | `agent.toml:48`, `config.rs:548/565` | only `validate()`'s `> 0.0` check | **INERT** — N-13 |
| `BinanceSourceConfig.rest_url` | `agent.toml:6`, `config.rs:310/317` | **never** — all 4 `BinanceFeed::new` sites pass ws\_url twice | **INERT** — N-14 |
| `SignalLogConfig.enabled` | `config.rs:623` (default false) | **nowhere** in `agent`/`ui`/`trader` src | **INERT** — N-15 |
| `SmaCrossoverConfig.enabled` | `agent.toml:30` (`true`) | **never** — registered unconditionally | **INERT** — N-16 |
| `Tcn/PatchTstOverlayMomentumConfig.base_config_path` | `tcn_overlay_momentum.rs:66`, `patchtst_overlay_momentum.rs:69` | **zero field accesses** (4 grep hits total) | **INERT** — N-17 |

### 5.2 LIVE — traced to the line that acts (no finding; recorded so the negative space is real)

**[SRC]** `ScenarioConfig`: `seed`, `write_report`, `data_source`, `bars_override`,
`initial_capital`, `short_enabled`, `sma_fast_len`/`sma_slow_len`, `composed_toml_override`,
`dvol_override`, `macro_regime_series`, `reports_dir`. **`latency_slippage_sim.venue_filter`
is LIVE post-#79** — applied at `engine.rs:2630`, `sma_composed_run.rs:489`, `:1103`.
`LatencySlippageSimConfig.slippage_model` and `volume_usd_per_symbol` are LIVE at
`scenarios/sim.rs:63` and `:79`.

`RiskLimits`: `per_symbol_exposure_cap` (`sizing.rs:68`, `portfolio.rs:150`,
`order.rs:164`), `price_sanity_band` (`order.rs:151`), `portfolio_exposure_cap`
(`portfolio.rs:189` — bug-log #69's inertness is fixed).

Agent config LIVE: `mode`, `parquet_root`, `replay_fast`/`replay_pace_ms`
(`runtime.rs:1038-1039`), `bus.*_capacity` (`bus.rs:105-108`), `audit.tick_bus_capacity`
(`cockpit_live.rs:277`), `audit.ledger_db_path`, `observability.prometheus_*`
(`observability.rs:107-122`), `cost.budget_usd_month` (`runtime.rs:913`), `funding.*`
(`runtime.rs:965-972`), `universe.usd{t,c}_enabled` (`cockpit_live.rs:342-343` →
`core/universe.rs:195`), `reflection.*` (`main.rs:140-165`), `advisor.eur_usd_rate*`
(`cockpit_live.rs:458-462`), all `llm.*`, `risk.sizing.fixed_fraction`,
`risk.per_symbol_exposure_cap`, `backtest.{slippage_bps,taker_fee_bps,initial_capital_usdt}`.

Strategy overlay params spot-verified LIVE: `vol_killswitch_overlay.rs:208,220,221,229`;
`regime_dispatcher.rs:269,277`; `drawdown_control_overlay.rs:231`; `vol_meanreversion.rs:196`.

### 5.3 Known stub, disclosed in-source (not counted as a finding)

**[SRC]** `crates/agent/src/runtime.rs:2952-2961` — `default_risk_telemetry_stub` hardcodes
`per_symbol_caps: HashMap::new()` and `daily_loss_cap_pct: 100` instead of reading
`config.risk`, so the cockpit Risk/Limits screen never displays configured caps. The
doc-comment at `:2939-2947` labels it "plumbing-only … actual risk-engine wiring lands as a
follow-up". Same reachability gap as N-3, from the display side, but honestly labelled.

---

## 6. Forked / duplicated execution paths

Two implementations of one concept, kept in sync by memory.

### F-1 — Short-leg execution: backtest (fixed) vs agent forward loop (unfixed) — **N-2**
- **Halves**: `sma_composed_run.rs:650-698` (engine-routed, venue-filtered) vs
  `agent/runtime.rs:2186-2267` (hand-synthesised `Fill`, `continue` past the engine).
- **What keeps them consistent**: nothing. The agent-side comment asserts they are the same
  and **is now false**.
- **Drift demonstrable**: yes — long legs pay slippage + lot-rounding in the forward loop,
  short legs pay only the taker fee.

### F-2 — Three ranked arms bypass `PaperEngine` entirely (closed-form equity curves)
**[SRC]** `crates/backtest/src/bakeoff/buyhold.rs`, dispatched from `engine.rs:2124`
(`v0.buyhold`), `:2262` (`v0.always_short`), `:2395` (`v0.macro_riskon`) — pure
`Vec<Decimal>` curve functions with no `Order::new`, no `engine.step`, no `MatchConfig`.
Self-documented at `buyhold.rs:141-143`: every transition executes at the bar's own close
with *"no taker fee, no slippage, no lot rounding and no min-notional filter"*.
- **Why it matters**: `v0.buyhold` is the **benchmark the crown is measured against**
  (`bakeoff/mod.rs:984 BUYHOLD_ID`). The frictionless half sits on the comparison baseline
  itself, while the arms it is compared to pay full friction. This is #80's asymmetry moved
  onto the reference leg.
- **Keeping them consistent**: nothing structural; `buyhold.rs:149-157` argues the gap is
  anchor-neutral and defers the fee to an operator decision. Bug-log #80's option **(B)**
  ("route through `PaperEngine`") is the same fix shape and has not been taken.

### F-3 — `venue_filter` applied at 3 of 13 `PaperEngine` construction sites; its sibling `sim_slippage_cost` at 10 of 13
**[SRC]** enumerated non-test construction sites:

| site | `.with_venue_filter_mode` | `sim_slippage_cost` |
|---|:--:|:--:|
| `backtest/src/engine.rs:2629` | ✅ | ✅ |
| `scenarios/sma_composed_run.rs:488` | ✅ | ✅ |
| `scenarios/sma_composed_run.rs:1102` | ✅ | ✅ |
| `scenarios/momentum.rs:280` | ❌ | ✅ |
| `scenarios/tcn_overlay.rs:148` | ❌ | ✅ |
| `scenarios/tcn_overlay_weights.rs:143` | ❌ | ✅ |
| `scenarios/patchtst_overlay_weights.rs:150` | ❌ | ✅ |
| `scenarios/garch_vol_target_overlay.rs:216` | ❌ | ✅ |
| `scenarios/regime_dispatcher.rs:241` | ❌ | ✅ |
| `scenarios/pairs.rs:150` | ❌ | ✅ |
| `scenarios/montecarlo.rs:196` (**anchored** θ-surfaces) | ❌ | ❌ |
| `scenarios/threshold_sweep.rs:151` (**anchored**) | ❌ | ❌ |
| `agent/src/runtime.rs:2094` (**forward paper loop**) | ❌ | ❌ |

- **The fork**: one config struct (`LatencySlippageSimConfig`) with **two independent
  consumption sites** — the cost half (a free function read off the struct) and the filter
  half (an opt-in builder method on the engine). Nothing binds them.
- Bug-log #79 records the 9 non-advisor scenario runners as deliberately unchanged (frozen
  research lanes, callers pass `None`) — that is sound. **`agent/runtime.rs:2094` is not in
  that list** and is not a frozen research lane; it is the forward paper loop.
- Also **[SRC]** `pairs.rs:155` sets `portfolio_exposure_cap: Some(dec!(0.75))` where all
  nine siblings use `Some(dec!(0.50))` — an outlier with an in-line justification comment.

### F-4 — Four independent Sharpe/Sortino/Calmar implementations that disagree by construction
**[SRC]**

| impl | returns | stdev denom | annualisation |
|---|---|---|---|
| `backtest/src/scenarios/sma_composed.rs:28` `compute_sharpe` | **simple** `(w1−w0)/w0` | `n` (population) | `sqrt(525_600)` (minute) |
| `backtest/src/stats/mod.rs:40` `compute_sharpe_hourly` | **log** `ln(c/p)` | `n` | `92.601_295_098_46` |
| `forecast/src/bin/sharpe_comparison.rs:137` | log | `n` | same constant (private copy) |
| `reports/src/render/risk_metrics.rs:108` `sharpe` | simple | **`n−1` (sample)** | `MINUTES_PER_YEAR / cadence` |

- **A false claim of uniqueness holds them together**: `backtest/src/lib.rs:142` states
  *"Single source of truth — `main.rs` calls `backtest::compute_sharpe` so there is no
  duplication"*, while `stats/mod.rs` ships a second, log-return one in the same crate.
- **Sortino diverges by more than a constant**: `stats/mod.rs:83` divides `Σ min(r,0)²` by
  **`n` (all returns)**; `risk_metrics.rs:132-136` filters to `r < 0` then divides by
  **`downside.len()`**. These differ by `sqrt(n_neg/n)` — a different statistic, not a
  rescaling. Calmar likewise: geometric CAGR (`stats/mod.rs:100`) vs arithmetic
  `mean × periods_per_year` (`risk_metrics.rs:151`).

### F-5 — Two dispatch tables over one arm-ID namespace: bake-off vs forward registry
**[SRC]** `backtest/src/engine.rs::run_scenario` (`:1084`–`:2573`) and
`agent/src/runtime.rs::build_registry_for` (`:335`–`:658`) each `match` on the same string
IDs and each decides what the arm *is*. Set diff:

| ID | bake-off (`run_scenario`) | forward (`build_registry_for`) |
|---|---|---|
| `v0.dvol_regime` | real DVOL series (`engine.rs:2059`) | `DvolRegimeStrategy::new(sym, vec![], …)` — **empty as-of, permanent warm-up** (`runtime.rs:597-604`) |
| `v0.buyhold` | closed-form curve | `AlwaysLongStrategy` through `PaperEngine` |
| `v0.always_short` | closed-form short curve | **`AlwaysLongStrategy`** + a `short_enabled` clamp inversion (`runtime.rs:451`) |
| `v0.macro_riskon` | `run_macro_gated_buyhold_path` | **`anyhow::bail!`** (`runtime.rs:637`) — the #81 fix |
| `v0.5.sma` | **absent** | present (`runtime.rs:359`) |
| `v1.momentum`, `v1.5a.mr`, `v1.5a.pairs` | present | **absent** (falls to `unknown => bail!`) |
| all others (8 singles + 8 `v0.8.vote.*` + 4 `_ls`) | present | present |

- **What keeps them consistent**: nothing structural. The `v0.macro_riskon` half was
  repaired *after* #81 by hand-writing a `bail!` in one arm — a per-arm patch, not a shared
  type. Each new arm must be added to two `match` blocks by memory.
- **`v0.always_short` reproduces #81's shape one arm over**: the ranking context runs a
  closed-form short curve; the execution context runs `AlwaysLongStrategy` and relies on an
  inversion clamp elsewhere in the loop — under one label. **[UNVERIFIED]** whether the two
  produce the same equity.

### F-6 — Two arm-existence gates over the same fact
**[SRC]** `arm_runs_in_this_build` (`bakeoff/mod.rs:186`, consumed by
`ui/leaderboard/runner.rs:87`) gates the **declared** count; `run_bakeoff`'s dispatch guards
(`bakeoff/mod.rs:~1167`, `~1204`) gate the **executed** field. Two independent gates over
"does this arm exist in this build", one in `ui`, one in `backtest`, held together by a
doc-comment instructing callers to remember. **This fork is exactly what N-1 falls through.**

### F-7 — Two conventions for the same `cfg(not(realdata))` situation; one takes the silent branch
**[SRC]** `crates/backtest/src/main.rs:309-315` — `build_volume_map_for_scenario`'s
non-`realdata` twin returns **`None` with no log line and no error**. The `None` flows to
`volume_usd_per_symbol`, and `main.rs:236` documents that an absent V makes fills *"hit
`MAX_SLIPPAGE_BPS`"*; the `realdata` half warns loudly about exactly this at `:280`.
Contrast the loud convention elsewhere in the same tree —
`bin/param_robustness_sweep.rs:546` `bail!`s, `main.rs:1356` and `:1456` return errors
naming the missing feature, and N-7's three scenarios `bail!` with the rebuild command.

### F-8 — Two report-writing stacks (LOW, no drift demonstrated)
**[SRC]** `crates/backtest/src/report/{sma,momentum,pairs,tcn_overlay,regime_dispatcher,yahoo}.rs`
each hand-format their own YAML front matter (5 copies of the `generated: {stamp}` block).
Separately `crates/reports/src/render/` renders the operator report from typed inputs. The
CSV schema is duplicated across crates **by comment only** — `engine.rs:803`: *"Schema is
identical to `reports::csv_artifacts::{write,read}_equity_csv`"*. The 119 body-SHA anchors
pin the `backtest` half's bytes; **nothing pins the two halves to each other.**

### Checked and clean (recorded so the survey is falsifiable)
- **[GREP]** No fourth arm list: `crates/ui/src/leaderboard/` and `bin/cockpit_live.rs:368`
  delegate to `agent::runtime::build_registry` / `backtest::bakeoff`.
- **[SRC]** `dvol_corpus_symbol` (`bakeoff/mod.rs:130`) — deliberately de-forked from three
  copies to one; `dvol_supported` derives from it so the allowlist cannot fork again.
- **[SRC]** `sim_slippage_cost` — single definition in `scenarios/sim.rs`, with a grep gate
  recorded at `momentum.rs:546`.
- **[CARGO]** `data/fixtures` is genuinely absent from every `-e normal` production
  resolution — the manifest's claim holds.

---

## 7. What I could not determine, and why

Stated plainly. A labelled unknown is worth more than a confident guess.

**Closed during the sweep** (recorded so the delta is visible):

- Whether the cockpit ranks the five short arms — **closed, it does not** (N-2b; exhaustive:
  two `field:` writers, one production `run_bakeoff` caller, one non-definition reference to
  `default_short_field`).
- The `#[allow(dead_code)]` census — **closed at 42**, and the *reason* for the compiler's
  silence closed with it: no global suppression exists; rustc's blindness to `pub` items in
  lib crates is the dominant cause (N-0).
- **N-1's build verification — closed, RED.** `cargo test -p ui --lib leaderboard::runner`
  returned `FAILED … v0.dvol_regime must be dispatchable in every build`. It closed by an
  unanticipated route: the `backtest` half was fixed by another session at 09:36 while the
  test was compiling, so the failure simultaneously confirms the original finding and proves
  the fix is half-landed (N-1c). The downstream double-subtraction (N-1d) was found by
  re-deriving the arithmetic after that change.

**Still open:**

1. **This map is a snapshot of a MOVING TREE, and it is already partly stale.**
   `crates/backtest/src/bakeoff/mod.rs` was edited by another session at **09:36:27**, mid-
   sweep (N-1's status note). Findings verified before that timestamp were re-checked only
   where N-1 led me; **N-2 through N-17 were verified between ~08:50 and ~09:34 and have not
   been re-confirmed against the current tree.** Anyone acting on this document should
   re-run the §Appendix commands first. The `claims-ledger-2026-08-15.md` in the same
   directory (owner `orchestrator`, 09:20) is not mine and I did not read it — if it
   overlaps, reconcile the two rather than assuming either is current.

2. **Whether `v0.dvol_regime`'s two implementations agree** (F-5): the bake-off runs a real
   as-of series, the forward registry a permanently-warm-up stub with `vec![]`. The comment
   at `runtime.rs:590-596` claims agreement; the two are computed by different machinery. I
   did not measure. **Unverified — needs a run.**

3. **Whether `BinanceFeed` ever issues a REST call** using the field it never receives
   (N-14). I verified the field is never passed; I did not trace every use of
   `self.rest_url` inside `crates/data/src/binance.rs` to establish the runtime consequence.
   The wiring gap is certain; the impact is **unverified**.

4. **Whether `crates/exec/src/paper.rs`'s engine reads its seed** (N-5c covers
   `crates/backtest`'s `PaperEngine` only). Not checked.

5. **The `--all-targets` half of the dead-code check** — and, given N-0, **it is worth less
   than I first assumed.** `cargo check --workspace` completed and is reported in N-0;
   `cargo check --workspace --all-targets` was still running past ~35 min and produced no
   output. I had wanted it to separate "dead everywhere" from "test-only reachable" by
   compiler evidence. But N-0 establishes that rustc does not report unused **`pub`** items
   in a library crate at all — and every Tier-1 item in §4.1 is `pub`. So that run would
   **not** have surfaced them under any configuration, and §4's grep-based method is not a
   fallback here, it is the *only* method that works. The residual value of running it is
   limited to private items in binary crates. §4 remains sound for every item listed but is
   **not exhaustive**: there may be dead items nobody thought to grep for.

6. **Non-`backtest` crates' internal reachability.** I mapped `agent`, `backtest`, `ui`,
   `exec`, `reports`, `data`, `risk` where the trail led. `features`, `cost`, `reflection`,
   `replay-cache`, `llm`, `trader` and `core` were **not** swept for dead public items.

7. **Runtime-gated (as opposed to compile-gated) capabilities.** This map covers
   `cfg(feature)` reachability. A capability disabled by a runtime `if cfg.enabled` that is
   never true is the same defect class (N-15/N-16 are two instances found incidentally) and
   was **not** systematically swept.

8. **Anchor impact of anything here.** Not assessed. Several findings touch
   `scenarios/montecarlo.rs` and `bin/threshold_sweep.rs`, which **do** write anchored
   bodies. `bash scripts/verify_anchors.sh` must run before and after any fix. This document
   changes no code and touches no `evidence/` file, so the gate is unaffected by writing it.

---

## Appendix — the reusable detector

The three commands that produced most of this map, for re-running after any manifest change:

```bash
# 1. What features does each workspace crate ACTUALLY get, per entry point?
#    -e normal excludes dev-deps; without it, test-only features look production-enabled.
cargo tree -p <entry-crate> -e normal --format '{p}||{f}' \
  | sed 's/^[^a-zA-Z0-9]*//' | grep ' v0.1.0' | sort -u

# 2. Does anything enable <crate>/<feature>?  A zero-hit result IS the finding.
grep -rn '<crate>/' --include=Cargo.toml .

# 3. Authoritative bin/test/feature inventory, including the required-features trap.
cargo metadata --no-deps --format-version 1
```

Then for every `cfg(feature)`-gated capability, in order:
1. Is it on in any shipping build? (command 1 — never the manifest)
2. If not, does the `cfg(not(...))` arm **bail** or return a plausible value?
   `bail!` = documented limitation. Bare `None` / empty series = defect.
3. Is there a test that asserts the feature is on? If the only proof is a constructor
   assertion, the capability does not exist (bug-log #79's moral).
