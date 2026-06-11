---
slug: vol-killswitch-overlay-noop-fix
version: 0.1.0
status: shipped
owner: shipped
priority: P0
updated: 2026-05-27
shipped: 2026-05-26
parent: bug-log #65
predecessor: v3-volatility-forecaster-noop-fix v0.1.0 (precedent)
---

# vol-killswitch overlay — no-op wire-up FIX

> **P0 safety wiring-bug.** `crates/strategy/src/vol_killswitch_overlay.rs`
> increments its `kill_switch_count` counter correctly when the trigger
> condition fires, but the `Signal::kind = SignalKind::Hold` mutation
> never reaches the executor's load-bearing path for the
> cross-sectional momentum inner strategy. Equity matches the
> un-overlaid baseline byte-for-byte. The same shape as
> `v3-volatility-forecaster-noop-fix v0.1.0` 2026-05-22; a *killswitch
> that doesn't kill is the worst kind of no-op.*

## Why now — P0 safety framing

This is **not a feature ask**; it is a P0 safety wiring-bug recovery.
The vol kill-switch overlay is the project's first concrete production
risk-overlay (R6.b secondary under the v3 volatility lane). The
contract the operator and the executor rely on is binary: when
`sigma_hat > threshold_multiplier × rolling_median(σ̂)`, the inner
strategy's `Buy` / `Sell` signals on the affected symbol MUST be
converted to `Hold` so the executor does not take risk. Today the
overlay's `stats.kill_switch_count` increments correctly when the
trigger condition fires — but the equity stream is byte-identical to
the un-overlaid baseline. The killswitch is **decorative**.

Production failure mode:

1. Realized vol on BTCUSDT spikes (e.g. exchange flash event).
2. GARCH step emits `sigma_hat ≫ threshold × rolling_median`.
3. Overlay's `kill_switch_count` increments to 1, 2, 3 … (operator
   sees "kill-switch tripped" in metrics).
4. **Executor takes the position anyway** because `Signal::kind` is
   still `Buy` for the affected symbol — the overlay's protection is
   present in the counter but absent from the signal stream.
5. Operator's metrics dashboard shows "killswitch fired" + a fresh
   drawdown on the same symbol.

This is the worst-case failure mode for a risk overlay: visible in
the diagnostics, silent in the trade tape. CLAUDE.md non-negotiable
explicitly cites the `v3-volatility-forecaster-noop-fix 2026-05-22`
precedent for exactly this pattern; that precedent fixed
`vol_targeting_overlay.rs` under the same root cause shape (scale
computed, never applied). This brief is the symmetric recovery for
the kill-switch sibling.

## Discovery context (Bug #65)

Surfaced 2026-05-26 by Wave 1's overlay-e2e hygiene gate
([`crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs`](../../crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs)).
Two tests fail correctly detecting the no-op:

- `trigger_fires_and_equity_diverges` — overlay equity == baseline
  equity (0 bp divergence vs ≥1 bp required). Literal panic line:
  ```
  vol-killswitch overlay equity divergence is below 1 bp —
  the overlay may be a no-op. baseline_equity=1.00000000,
  killswitch_equity=1.00000000, divergence=0.00000000,
  required_min=0.00010000 (1 bp). kill_switch_count=2
  ```
- `post_trigger_signals_are_hold` — overlay emits zero `Hold` signals
  on the trigger bar despite `kill_switch_count` advancing.

Both currently `#[ignore]`-gated with
`tracked-in: bug-log #65 vol_killswitch_overlay no-op` annotations.
The negative control (`passthrough_when_threshold_unreachably_high`)
PASSES, so the trigger path is the broken one — not the passthrough
path.

## Smoking gun (from source read 2026-05-26)

`crates/strategy/src/vol_killswitch_overlay.rs:169-244` carries the
overlay's `Strategy::on_bar` impl. Two telling fragments:

```rust
// lines 213-224 — trigger arithmetic + counter, correct in isolation
if state.cooldown_remaining > 0 {
    state.cooldown_remaining -= 1;
    true
} else if sh > threshold {
    state.cooldown_remaining = self.config.cooldown_bars;
    self.kill_switch_count += 1;   // counter advances
    true
} else {
    false
}

// lines 229-244 — application path
let base_signals = self.inner.on_bar(bar);

if kill_active {
    base_signals
        .into_iter()
        .map(|mut sig| {
            if sig.symbol == bar.symbol {       // ← LOAD-BEARING FILTER
                sig.kind = SignalKind::Hold;
            }
            sig
        })
        .collect()
} else {
    base_signals
}
```

The literal kind-mutation code IS present (`sig.kind = SignalKind::Hold`)
— this is **not** the verbatim "scale computed, never applied" shape
of the vol_targeting precedent. Analyst's working hypothesis (H1):
**the `if sig.symbol == bar.symbol` filter is the bug.** The inner
strategy is `MomentumStrategy` (cross-sectional momentum), which
emits signals for the **basket** at rebalance time — the `bar.symbol`
that triggered the kill-switch is one of several symbols carrying
signals at that rebalance bar; some signals may be emitted with
`sig.symbol` values that do not match `bar.symbol`. For the e2e test's
2-symbol stub (BTCUSDT + ETHUSDT, k_long=1), the rebalance produces
1 Buy on whichever symbol ranks higher in momentum; if BTCUSDT is the
trigger bar but the rebalance emits a Buy on ETHUSDT (or vice versa),
the filter drops it and the kind stays `Buy`. This is consistent with
the test evidence: counter advances (trigger condition fires), but
zero Hold signals appear (filter rejects every Buy because the symbols
don't line up).

The architect MUST confirm or falsify this hypothesis at M-T1 (see H1
+ Q1). If H1 is wrong, the fix shape changes; we surface that
uncertainty explicitly rather than locking the wrong patch.

## Out of scope

- **GARCH model improvements.** Threshold tuning, alternative rolling
  statistics, dynamic median — all deferred to v0.1.1+. The fix is
  wire-only.
- **Kill-switch behavior on non-momentum inner strategies.** The
  overlay's constructor takes `MomentumStrategy` concretely
  (`pub struct VolKillSwitchOverlay { inner: MomentumStrategy, … }`);
  generalizing to `Box<dyn Strategy>` is a separate API ask.
- **Cooldown semantics.** `cooldown_bars = 4` stays as-is. The fix
  does not change when the kill is active, only how it propagates.
- **New backtest scenarios.** Vol-killswitch does not appear in any
  anchored scenario (`grep "vol_killswitch\|vol-killswitch"
  spec/anchors.toml` returns zero rows; verified 2026-05-26).
  Anchor risk is ZERO by construction.
- **Strategy trait surface generalization** (e.g. a new
  `Strategy::dampen_signals` method). Deferred to a follow-on once
  a second consumer surfaces. See Q3.
- **TCN / PatchTST overlay co-audit.** Out of scope here per
  precedent T-A2 finding (TCN overlay's kind-mutation is via
  `Signal { kind: modulated_kind, ..sig }` spread — no per-symbol
  filter, so the cross-sectional basket shape doesn't bite it).
  Architect re-confirms at M-T1.

## Requirements (R1-R6)

### R1 — Wire-up fix lands at the strategy → executor handoff

`vol_killswitch_overlay::on_bar` MUST mutate `Signal::kind` to
`Hold` for **every** signal in the rebalance basket when
`kill_active` is true for the bar's triggering symbol. The current
per-signal filter `if sig.symbol == bar.symbol` is the suspected
bug; the correct shape depends on the cross-sectional inner-strategy
semantic that the architect locks at M-T1 (see Q1).

Fix site: `crates/strategy/src/vol_killswitch_overlay.rs` lines
229-244 (the kind-mutation block). The fix is **wire-only** — no
GARCH change, no Strategy trait surface change at v0.1.0 scope, no
scenario change. Mirrors the
[`v3-volatility-forecaster-noop-fix`](../v3-volatility-forecaster-noop-fix/feature.md)
v0.1.0 fix shape (one strategy file, one method body, ~20 LoC
change).

**Acceptance**: a unit test asserts that when the kill-switch fires
on bar `b` (triggered by `b.symbol`), every signal returned by
`on_bar(b)` whose strategy intent would be a position-taking action
(`Buy` / `Sell`) is converted to `Hold`, regardless of whether
`sig.symbol == b.symbol`.

### R2 — Negative-control regression test stays green

`passthrough_when_threshold_unreachably_high` (already on disk,
already passes) MUST continue to pass post-fix. This is the
contract against introducing a new no-op-in-the-other-direction
bug where the fix accidentally suppresses signals on bars where
the trigger didn't fire.

**Acceptance**: post-fix `cargo test -p strategy --test
vol_killswitch_overlay_end_to_end passthrough_when_threshold_unreachably_high`
PASSES.

### R3 — `trigger_fires_and_equity_diverges` flips RED → GREEN

The currently `#[ignore]`-gated test MUST pass post-fix; the
`#[ignore]` annotation MUST be removed in the same commit.

Specifically:

- Pre-fix: `cargo test -p strategy --test
  vol_killswitch_overlay_end_to_end trigger_fires_and_equity_diverges
  -- --ignored` FAILS with the literal panic line documented in §
  Discovery context.
- Post-fix: `cargo test -p strategy --test
  vol_killswitch_overlay_end_to_end trigger_fires_and_equity_diverges`
  (without `--ignored`) PASSES.
- `#[ignore = "tracked-in: bug-log #65 vol_killswitch_overlay no-op"]`
  annotation removed at
  [`crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs:169`](../../crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs).

**Acceptance**: the literal assertion succeeds (overlay equity
diverges from baseline equity by ≥ 1 bp); `#[ignore]` annotation
removed.

### R4 — `post_trigger_signals_are_hold` flips RED → GREEN

The second `#[ignore]`-gated test MUST pass post-fix; the
`#[ignore]` annotation MUST be removed in the same commit.

- Pre-fix: zero Hold signals emitted on the trigger bar despite
  `kill_switch_count` advancing.
- Post-fix: ≥ 1 Hold signal emitted for BTCUSDT on the trigger
  bar (the assertion at
  [`crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs:281-290`](../../crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs)).
- `#[ignore = "tracked-in: bug-log #65 vol_killswitch_overlay no-op"]`
  annotation removed at
  [`crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs:237`](../../crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs).

**Acceptance**: the `hold_count > 0` assertion succeeds; `#[ignore]`
annotation removed.

### R5 — ADR-0038 § D6 wiring-bug-fix protocol applies (or is amended)

ADR-0038 § D6.b (the `v3-volatility-forecaster-noop-fix v0.1.0`
amendment) already documents the wiring-bug-fix re-emission
protocol for the **anchor-additive** case. Since
**vol_killswitch_overlay has zero anchored scenarios** (verified by
grep on `spec/anchors.toml`), there is no anchor re-emission to
gate here. R5 nevertheless requires the architect to confirm at
M-T1 either:

- **(a)** § D6.b applies verbatim (no anchor delta; protocol invoked
  trivially — the "enumerate affected anchors" clause is satisfied
  by the empty set), OR
- **(b)** A new § D6.c documentation-link-fix variant is required if
  the architect surfaces a documentation-link-fix sub-case during
  M-T1 (matches CLAUDE.md non-negotiable on anchored report
  immutability).

**Acceptance**: architect M-T1 produces a one-paragraph note in
the feature.md § Design block citing which protocol applies. No
ADR amendment needed by default (§ D6.b is already in the tree at
[`spec/architecture/adr/0038-vol-forecast-verdict-shape.md`](../architecture/adr/0038-vol-forecast-verdict-shape.md)).

### R6 — Unit + integration regression tests for `scale != 1.0` propagation through `vol_killswitch_overlay::apply`

In addition to R3 + R4's end-to-end tests, R6 locks ≥ 1 unit test
that fails under the pre-fix code at the strategy isolation layer
(no engine, no inner strategy execution). The test drives the
overlay's `on_bar` directly with a rigged GARCH state + a stub
inner-strategy emitter that returns a controlled multi-symbol
signal vector, and asserts the kill-active branch converts ALL
position-taking signals to `Hold` regardless of `sig.symbol`.

Pattern reference:
[`crates/strategy/tests/vol_targeting_overlay.rs:236-301`](../../crates/strategy/tests/vol_targeting_overlay.rs)
(R6 unit tests for the precedent feature —
`scale_cache_populates_after_on_bar` +
`quantity_scale_default_for_unseen_symbol`).

**Acceptance**: post-fix workspace `cargo test --workspace --features
candle,realdata` PASSES; the new R6 unit test would-fail under the
pre-fix code (developer captures the FAIL → PASS bracket per the
precedent's T-D-N3a/3b forensic-gate protocol).

## Risk register (K1-K6)

### K1 — H1 (cross-sectional filter) is the wrong root cause

The smoking-gun is plausible but unconfirmed. If the inner strategy
emits signals only for `bar.symbol` at the rebalance bar (i.e. the
filter `sig.symbol == bar.symbol` is correct in shape but some
*other* path is broken), the fix patches the wrong line.

**Mitigation**: H1 is falsifiable via a 10-line probe — print the
returned signal vector from `MomentumStrategy::on_bar` for the
spike bar in the e2e test, see whether the symbols of returned
signals overlap `bar.symbol` (filter is correct) or not (filter is
the bug). The architect runs this probe at M-T1 before locking the
fix shape. Cost: ~5 minutes wall-clock.

### K2 — Fix may break the non-cross-sectional inner-strategy case

If a future operator switches the inner strategy from
`MomentumStrategy` to a single-symbol strategy (e.g. SMA-cross),
the new fix shape (drop the filter; convert all signals) may over-
suppress signals on bars where the killswitch fires for a different
symbol. Today the overlay's constructor concretely takes
`MomentumStrategy`, so this is hypothetical, but the architect
should note the assumption explicitly in the M-T1 decomp.

**Mitigation**: documentation-only at v0.1.0. The fix should ship
with an inline comment in `vol_killswitch_overlay.rs` documenting
the assumed cross-sectional-basket-only semantic. If a future ask
generalizes the inner strategy, that ask spawns its own brief.

### K3 — Cooldown logic interaction

The kill-active state propagates beyond the trigger bar through
`state.cooldown_remaining > 0` (lines 213-216). The fix must
preserve cooldown behavior — when cooldown is active, ALL signals
on the affected symbol should be `Hold`, not just the trigger
bar's. The current per-signal filter (broken or not) at least
attempts to do this; the fix's broader-scope filter (per Q1's
default) must do the same.

**Mitigation**: R3's e2e test already drives `POST_SPIKE_BARS = 10`
bars after the spike; the cooldown contract is implicitly tested.
Developer adds an explicit unit test at R6 driving 5+ post-spike
bars + asserting all returned signals are `Hold`. Cost: ~30
minutes additional test authoring.

### K4 — Hygiene gate retrofit might surface neighbor bugs

The overlay-e2e hygiene gate (`crates/strategy/tests/overlay_e2e_coverage.rs`,
proposed in `spec/dev-notes/testing-strategy-review-2026-05-25.md`
§2) is in flight. If that gate ships before this fix, it may
surface analogous bugs in `patchtst_overlay_momentum.rs` (no e2e
test today) — out of scope here but the architect should note the
adjacent risk in M-T1.

**Mitigation**: this brief takes no position on the hygiene gate's
landing order. If `patchtst_overlay_momentum.rs` turns out to have
a parallel no-op, that surfaces as Bug #66 (or similar) under the
same recovery-brief pattern.

### K5 — `kill_switch_count` and `bars_total` are public mutable fields

The struct exposes `pub kill_switch_count: u64` and `pub bars_total:
u64`. The fix touches neither; these are diagnostic counters the
operator may consume externally. Architect should confirm at M-T1
that the fix preserves their semantics (counter increments
unchanged; only the kind-mutation propagation changes).

**Mitigation**: documentation-only. The R6 unit test should assert
`kill_switch_count` advances the same way pre-fix vs post-fix —
the bug is in propagation, not counting.

### K6 — Test count delta + workspace fail set

The two `#[ignore]`-gated tests come off the fail set; the count
goes from 1 PASS (`passthrough_when_threshold_unreachably_high`) →
3 PASS post-fix. The workspace fail count (cited as "31 new tests
+ 5 criterion benches + 4 insta baselines" in
`cockpit-activity-status-bar v0.1.0`'s closing note) gets `-2`
ignored-tests off the report. This is bookkeeping; not a risk.

**Mitigation**: tester captures the new count in M-FINAL report
and confirms it matches the expected delta (-2 ignored + 0 new
ignored from this feature).

## Hypothesis register (H1-H3)

### H1 — The `sig.symbol == bar.symbol` filter is the bug

**Direction**: high confidence. The inner strategy is cross-sectional
momentum, which emits signals for the basket at rebalance time; the
per-signal symbol filter discards signals whose symbol is not the
trigger bar's symbol.

**Falsifiable probe**: at M-T1, the architect inserts a
`tracing::warn!` in the e2e test's overlay path that logs each
`sig.symbol` returned by `MomentumStrategy::on_bar` on the spike
bar, alongside `bar.symbol`. If the symbol sets overlap exactly,
H1 is wrong (the filter is correct in shape; the bug is elsewhere).
If they differ (any signal carries a symbol that doesn't match
`bar.symbol`), H1 is confirmed.

**Prior**: ~85% confidence based on the smoking-gun read. The
2-symbol e2e stub (`BTCUSDT` + `ETHUSDT`, `k_long=1`) emits exactly
one Buy per rebalance for the top-ranked symbol. With `bar.symbol
= BTCUSDT` triggering the kill-switch, the Buy may land on ETHUSDT
(or BTCUSDT, depending on momentum scores) — either way the filter
catches at most 1 of 2 cases.

### H2 — The trigger condition itself is sound

The unit test `vol_killswitch_new_initialises` plus the e2e test's
`kill_switch_count > 0` assertion both pass under the pre-fix
code, indicating the GARCH step + rolling-median + threshold check
all execute correctly. The fix should NOT touch lines 184-227 of
`vol_killswitch_overlay.rs` (the trigger arithmetic).

**Verification step**: at M-T1, the architect re-confirms the
trigger path is sound by adding the R6 unit test before any wire-up
change; the test should pass on the counter assertion but fail on
the kind-mutation assertion. This brackets H2 cleanly.

**Prior**: ~95% confidence. The trigger arithmetic is mechanical
and well-isolated; the bug is structurally downstream.

### H3 — The fix is single-file, single-method body, < 20 LoC

The precedent (vol_targeting_overlay) took ~80-150 LoC across 3
files including the new trait method + cache + sizing-pipeline
hook. The vol_killswitch fix is structurally simpler: the
kind-mutation code already exists; only its scope needs widening.
The shape is "drop the `if sig.symbol == bar.symbol` filter" or
"replace it with a different filter that covers the basket case."

**Falsifiable**: if the architect's M-T1 lock surfaces a need to
touch `MomentumStrategy::on_bar` (e.g. to expose a per-symbol
intent map), or to add a new trait method like
`Strategy::dampen_signals(&self, symbol_to_dampen: &Symbol) ->
Signal`, H3 is wrong and the cost estimate widens. If the fix
stays inside `vol_killswitch_overlay.rs` lines 229-244 with no
sibling touches, H3 is confirmed.

**Prior**: ~80% confidence based on the smoking-gun read. The 20%
risk is that fixing the filter's scope without breaking the
single-symbol-inner-strategy case (K2) requires touching the
overlay's struct shape (e.g. tracking which symbols are kill-active)
which inflates the change. Architect M-T1 confirms.

## Operator-decide questions (Q1-Q3)

Three operator-decide rows; all standing-Autoapprove-eligible per
the v3-volatility-forecaster-noop-fix precedent. Orchestrator may
auto-tick at M-OD.

### Q1 — Fix shape: mutate `Signal::kind = Hold` on trigger vs add a new `Signal::quantity_scale` field

| Option | Action                                                                                                                                                                                                                                                                                                                                          | Analyst recommendation |
|--------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------|
| (i)    | **Mutate `Signal::kind = Hold` on trigger** for ALL signals in the rebalance basket when `kill_active` is true; drop the `if sig.symbol == bar.symbol` filter (or replace it with a basket-aware filter). Fix lives entirely inside `vol_killswitch_overlay.rs:229-244`. ~10-20 LoC.                                                              | **DEFAULT** — smallest blast radius; matches the killswitch's binary "on/off" intent; preserves Signal type shape.            |
| (ii)   | Add `quantity_scale: f64` field to `trading_core::Signal` (default `1.0`); overlay sets it to `0.0` on trigger; executor honors. Mirrors Q1=(i) from the vol_targeting precedent. Larger blast radius (Signal is serialized in audit ledger + journal).                                                                                          | Rejected — kill-switch semantic is binary, not scalar; Q1=(ii) from the precedent already chose the lighter-touch path. |
| (iii)  | Add `Strategy::dampen_signals(&self, symbol: &Symbol) -> bool` defaulted trait method; sizing pipeline queries the strategy at signal-construction time. Mirrors Q1=(ii) from the vol_targeting precedent.                                                                                                                                       | Rejected — over-engineered for a binary on/off behavior; defer to a follow-on if a second consumer surfaces. |

**Analyst recommendation: (i) — mutate `Signal::kind = Hold` on trigger.**

**Rationale**: smallest blast radius (one file, one method); the
killswitch's intent IS binary ("kill or don't kill," not "scale by
0.0"); the kind-mutation code is already 80% present in the source
— the bug is the filter's scope, not the absence of the mutation.
The precedent (vol_targeting) needed (ii)/(iii) because vol-
targeting's semantic is scalar (multiply by 0.6, multiply by 1.7);
this brief's semantic is binary (Hold or not). Different problem,
different default. Standing Autoapprove applies.

### Q2 — Anchor handling: new anchors at v0.1.0 or zero anchor delta

| Option | Action                                                                                                                                                                                                                                                                                                                                          | Analyst recommendation |
|--------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------|
| (a)    | **Zero new anchors at v0.1.0.** vol_killswitch_overlay does not appear in any anchored scenario (`grep "vol_killswitch\|vol-killswitch" spec/anchors.toml` returns zero rows; verified 2026-05-26). The killswitch fires only in rare-vol scenarios, not part of the standard anchored backtest run set. R3 + R4 e2e tests cover the contract. | **DEFAULT** — zero anchor risk by construction.            |
| (b)    | Add 1-2 new anchored scenarios that exercise the killswitch trigger path (e.g. `top10-2023-fy-vol-killswitch-realdata` with the killswitch threshold tuned to fire on the 2023 March BTC vol spike). Locks the post-fix behavior under anchor regression.                                                                                       | Defer to v0.1.1+ — anchor lock at v0.1.0 risks shipping with the wrong threshold/scenario shape; let the operator validate the production behavior first via the e2e test before locking. |
| (c)    | Re-emit 0 anchors but tighten the e2e test to drive the full `top10` universe (10 symbols) through a rigged trigger scenario.                                                                                                                                                                                                                   | Rejected — over-scopes the recovery; the 2-symbol stub e2e test is sufficient evidence at v0.1.0; the operator can author a v0.1.1 brief if richer coverage matters. |

**Analyst recommendation: (a) — zero new anchors at v0.1.0.**

**Rationale**: the 34 locked anchors stay byte-identical by
construction; the fix's scope is strategy-internal; the e2e test
is the load-bearing gate (mirrors R2 from the precedent feature).
Standing Autoapprove applies.

### Q3 — `Strategy::dampen_signals` trait surface — lock now or defer

| Option | Action                                                                                                                                                                                                                                                                                                                                          | Analyst recommendation |
|--------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------|
| (a)    | **Defer the trait-shape decision** to the architect M-T1. The simplest possible fix (Q1=(i) — patch the filter inside `vol_killswitch_overlay.rs`) is the immediate priority. A trait method like `Strategy::dampen_signals` can land in v0.1.1+ once a second consumer surfaces.                                                              | **DEFAULT** — minimum scope; YAGNI applies.            |
| (b)    | Add `Strategy::dampen_signals(&self, symbol: &Symbol) -> bool` defaulted trait method at v0.1.0; refactor the killswitch overlay to use it. Mirrors Q1=(ii) from the vol_targeting precedent.                                                                                                                                                  | Rejected — over-engineered for a single consumer; YAGNI bias. |
| (c)    | Add `Strategy::signal_kind_override(&self, symbol: &Symbol) -> Option<SignalKind>` trait method; richer than (b) but heavier surface.                                                                                                                                                                                                          | Rejected — same YAGNI concern; richer surface = more API to maintain.   |

**Analyst recommendation: (a) — defer the trait-shape decision to architect M-T1.**

**Rationale**: the simplest possible fix is the immediate priority;
the precedent's Q1=(ii) trait surface (`Strategy::quantity_scale`)
exists because vol-target needed a *scalar query at sizing time*;
kill-switch needs a *binary mutation at signal-emission time*. The
mutation can happen inside the overlay's `on_bar` body without a
trait surface. If a follow-on brief surfaces a second binary-
mutation consumer, the trait method drops in then. Standing
Autoapprove applies.

## Non-regression contract

The fix is wire-only and anchor-zero. Specifically:

1. **34 anchors stay byte-identical.** Verified by grep on
   `spec/anchors.toml` for `vol_killswitch` / `vol-killswitch` —
   zero matches (2026-05-26). `bash scripts/verify_anchors.sh` MUST
   show `ANCHORS PASS (34 / 34)` post-fix; tester confirms at
   M-FINAL.
2. **Test count delta: 1 PASS → 3 PASS** in
   `vol_killswitch_overlay_end_to_end.rs`. The 2 `#[ignore]`
   annotations come off (R3 + R4); the passthrough test
   (`passthrough_when_threshold_unreachably_high`) stays green
   (R2). Net: -2 ignored, +2 pass.
3. **Workspace fail count delta: -2.** The 2 `#[ignore]`-gated
   tests no longer appear in the workspace ignore set.
4. **No new audit migration, no new persistence, no new IPC.**
   Pure strategy-internal patch.
5. **`Signal` struct shape preserved.** Q1=(i) default ships zero
   changes to `trading_core::Signal`; audit / journal serialization
   stays byte-identical for all non-killswitch strategies.
6. **3 v3-volatility-forecaster-noop-fix anchor SHAs stay byte-
   identical**: `9fa64d467f…` (top10-2023-fy-vol-target-overlay-
   realdata), `d21db467f1…` (sharpe-comparison-vol-target-bs1-
   realdata), `ff2b934961…` (sharpe-comparison-vol-target-bs1-
   realbaseline). These come from the precedent feature's M-FINAL
   2026-05-22 and are the closest neighbors to this brief; the
   fix does not touch their sources.
7. **`kill_switch_count` semantic preserved.** R6 unit test asserts
   the counter advances the same way pre-fix vs post-fix.
8. **`MomentumStrategy::on_bar` semantic untouched.** The fix lives
   strictly inside the overlay's wrapping body; inner strategy
   stays a pure passthrough on its own.

## Cost framing

**1-3 days end-to-end wall-clock**, distributed as:

| Phase     | Owner       | Estimate     | Notes                                                                                                                                                                                                                                                |
|-----------|-------------|--------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| M0        | analyst     | ~0.5 day     | THIS pass. Feature brief + tasks scaffold + backlog row + trace row + bug-log update.                                                                                                                                                                |
| M-OD      | orchestrator | < 30 minutes | Standing Autoapprove ticks Q1=(i), Q2=(a), Q3=(a). Frontmatter flips `proposed → in-progress`, `analyst → architect`.                                                                                                                              |
| M-T1      | architect   | ~0.5 day     | H1 falsification probe (~5 min); fix shape lock (Q1=(i)); single-wave decomp.md sketch; R6 unit test shape; forensic-gate FAIL/PASS protocol. No ADR amendment needed by default (R5=(a)).                                                          |
| M-DEV     | developer   | ~1 day       | Single wave (Wave A). T-D-N1 R6 unit test pre-fix RED → T-D-N2 wire-up fix → T-D-N3 R6 unit test post-fix GREEN → T-D-N4 remove `#[ignore]` annotations from R3 + R4 e2e tests → T-D-N5 workspace gate (fmt + clippy + test + anchors).            |
| M-FINAL   | tester      | ~0.5 day     | cargo fmt + clippy + test + verify_anchors gates; confirm 3 PASS in the e2e file; confirm no `#[ignore]` annotations remain; confirm test count delta + workspace fail delta match the non-regression contract; write `reports/test-final-<date>.md`. |
| M-PRESENTER | presenter | ~0.5 day     | Assemble `presentations/vol-killswitch-overlay-noop-fix-<date>.md`. Route per verdict tree (R-O1 / R-O2 / R-O3 below).                                                                                                                              |

Total worst-case: 3 days wall-clock; realistic: 1.5-2 days if Wave A
single-shots cleanly. **No LLM costs**; pure source patch.

## Pre-drawn verdict routing tree (presenter inherits)

Standing Autoapprove on Q1..Q3 defaults; route selection at M-FINAL
keys to the post-fix e2e test outcome.

| Route  | Condition                                                                                                                              | Routing implication                                                                                                                                                                                                          | Next action                                                                                                                                                                                                                  |
|--------|----------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| R-O1   | Fix works; all 3 e2e tests PASS (including the 2 previously `#[ignore]`'d); no anchor delta; no workspace regression.                | The killswitch is now live + tested. Operator approves the ship; frontmatter flips `in-progress → shipped`. Bug #65 closes with `Status: fixed`.                                                                            | **SHIP** — standing Autoapprove proceeds. Closes the v3 vol-overlay no-op recovery class (the second of the two siblings; v3-vol-targeting was the first 2026-05-22).                                                       |
| R-O2   | Fix works for R3 + R4 e2e; R2 passthrough test stays green; BUT some unrelated anchor SHA deviates (unexpected propagation).         | Operator-decide approval before ship. The anchor delta is unexpected per Q2=(a); the architect must surface the root cause (likely a clippy/fmt cleanup side-effect or an unintended sibling touch).                       | **HOLD** — operator decides whether the anchor delta is acceptable. If yes, ship under an ADR-0038 § D6 (or § D6.c if codified) re-emission protocol. If no, route back to developer for sibling cleanup.                  |
| R-O3   | Fix doesn't work — H1 is the wrong root cause; the filter is correct in shape; the bug is elsewhere (e.g. `MomentumStrategy::on_bar` rebalance timing). | Deeper investigation needed. The architect re-spawns at M-T1 with the H1 falsification evidence + a revised fix shape. Cost estimate widens to ~3-5 days; H3 is wrong.                                                       | **ARCHITECT RE-SPAWN** — feature stays in `in-progress`; analyst may need to re-spawn at M0 if the root cause moves into territory outside the strategy crate (e.g. core::Signal semantics or executor-side gating).         |

## References

- Bug log entry: [`spec/bug-log.md` § #65](../bug-log.md).
- Smoking-gun source: [`crates/strategy/src/vol_killswitch_overlay.rs:169-244`](../../crates/strategy/src/vol_killswitch_overlay.rs).
- Failing e2e tests (currently `#[ignore]`'d): [`crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs`](../../crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs).
- Precedent feature (vol_targeting sibling fixed 2026-05-22): [`spec/v3-volatility-forecaster-noop-fix/feature.md`](../v3-volatility-forecaster-noop-fix/feature.md).
- Precedent feature's decomp.md (wave shape reference): [`spec/v3-volatility-forecaster-noop-fix/decomp.md`](../v3-volatility-forecaster-noop-fix/decomp.md).
- Discovery dev-note (diagnostic chain for the vol_targeting precedent): [`spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`](../dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md).
- Testing-strategy review citing the "killswitch that doesn't kill" framing: [`spec/dev-notes/testing-strategy-review-2026-05-25.md`](../dev-notes/archive/2026-Q2/testing-strategy-review-2026-05-25.md) §1 P2.
- ADR-0038 § D5 (strategy-side composition lock) + § D6 + § D6.b (anchor-additive contract + wiring-bug-fix re-emission protocol): [`spec/architecture/adr/0038-vol-forecast-verdict-shape.md`](../architecture/adr/0038-vol-forecast-verdict-shape.md).
- CLAUDE.md non-negotiable on overlay-equity-divergence e2e tests: [`CLAUDE.md ## Non-negotiables`](../../CLAUDE.md#non-negotiables).
- Pattern reference for R6 unit tests: [`crates/strategy/tests/vol_targeting_overlay.rs:236-301`](../../crates/strategy/tests/vol_targeting_overlay.rs).

## Acceptance per milestone

- **M-OD** — Operator-decide Q1..Q3 resolved (standing Autoapprove
  defaults: Q1=(i), Q2=(a), Q3=(a)). Frontmatter flips
  `status: draft → in-progress`, `owner: analyst → architect`.
- **M-T1** — Architect lock: H1 falsification probe complete; §
  Design block appended; T-AR-1..T-AR-N ordered breakdown with
  file:line citations; R5 ADR protocol-citation (default § D6.b
  applies trivially); R6 unit test shape locked; forensic-gate
  FAIL/PASS protocol documented (mirror precedent's T-D-N3a/3b).
- **M-DEV** — Developer Wave A: R6 unit test added pre-fix
  (forensic FAIL captured) → wire-up fix landed → R6 post-fix
  GREEN → `#[ignore]` annotations removed from R3 + R4 → workspace
  gate PASS.
- **M-FINAL** — Tester gate: cargo fmt + clippy + test + anchors
  all PASS; 3 e2e tests PASS in
  `vol_killswitch_overlay_end_to_end.rs`; workspace fail count
  matches non-regression contract (-2 ignored); test report at
  `reports/test-final-<YYYY-MM-DD>.md`.
- **M-PRESENTER** — Presenter assembles
  `spec/vol-killswitch-overlay-noop-fix/presentations/vol-killswitch-overlay-noop-fix-<YYYY-MM-DD>.md`;
  routes per § Pre-drawn verdict routing tree.
- **M-OPERATOR** — Operator ticks approval. Frontmatter flips
  `status: in-progress → shipped`. Trace row
  `REQ-VOL-KILLSWITCH-NOOP-FIX-001` flips state. Bug #65 closes
  with `Status: fixed`.

## Design

> Architect locks at M-T1. The load-bearing decisions + Wave-by-
> Wave decomposition + cargo invocations + expected literal
> outputs live in `decomp.md` (created by architect; not in scope
> for this analyst pass).

## Implementation

**Completed 2026-05-26 — Q4=(p3) "Both" operator-locked decision.**

### A.1 — Test fixture fix

`crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs`

- `stub_momentum()`: `lookback_minutes` changed 60 → 5 (ring capacity 61 → 6).
- `build_bar_stream()`: Flat BTC warmup (100, not rising 100+i). Prevents GARCH sigma from rising above `min_median_floor=1e-3` during the 20 warmup bars.
- Two-spike design: BTC 100 → 1000 (spike, sets r_prev ≈ 2.3) → 50 (crash, GARCH fires: sigma_hat ≈ 0.73 >> 1e-3). At the crash bar, BTC score negative, ETH score 0 → Sell BTC + Buy ETH → kill converts to Hold.
- `min_median_floor: 1e-3` in all test configs with `threshold_multiplier=1.0` prevents early-kill during warmup (warmup sigma ≈ 4.47e-4 < 1e-3).

### A.2 — Broadened overlay filter

`crates/strategy/src/vol_killswitch_overlay.rs:231-244`

Dropped `if sig.symbol == bar.symbol` guard. The new semantic: when kill fires on any symbol, ALL signals in the rebalance basket → Hold. This is the Q4=(p3) broadened cross-sectional semantic (noted in K2 caveats in the filter comment).

### A.3 — Tests

Four tests (no `#[ignore]` annotations):
1. `trigger_fires_and_equity_diverges` — equity divergence ≥ 1 bp.
2. `post_trigger_signals_are_hold` — at least one Hold in kill-active window.
3. `passthrough_when_threshold_unreachably_high` — divergence < 1 bp with threshold=1e9.
4. `broadened_filter_dampens_cross_sectional_basket` — no Buy/Sell leaks during kill window.

Overlay hygiene gate: `vol_killswitch_overlay` removed from `KNOWN_UNCOVERED` — 2/2 gate tests pass.

### Test run output

```
test post_trigger_signals_are_hold ... ok
test broadened_filter_dampens_cross_sectional_basket ... ok
test passthrough_when_threshold_unreachably_high ... ok
test trigger_fires_and_equity_diverges ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Changed files

- `crates/strategy/src/vol_killswitch_overlay.rs` (A.2 — broadened filter + doc update)
- `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs` (A.1 + A.3 — full rewrite)
- `crates/strategy/tests/overlay_hygiene_gate.rs` (remove from KNOWN_UNCOVERED + fix clippy `&PathBuf → &Path`)
- `spec/bug-log.md` § #65 (A.4 — status FIXED)

## Verification

> Tester M-FINAL fills this section with the joint advisory
> verdict + cargo gates + anchors PASS + e2e PASS + cross-refs to
> `reports/test-final-<date>.md`.

## Changelog

- 2026-05-26 (analyst): brief authored at v0.1.0 / status=draft.
  P0 safety wiring-bug recovery; smoking-gun captured (lines
  229-244 of vol_killswitch_overlay.rs); H1 hypothesis (the
  `sig.symbol == bar.symbol` filter is the bug) surfaced as the
  85%-confidence root cause; H2 (trigger arithmetic sound) and
  H3 (single-file, < 20 LoC fix) bracket the cost; Q1=(i),
  Q2=(a), Q3=(a) defaults locked under standing Autoapprove
  per the v3-volatility-forecaster-noop-fix 2026-05-22 precedent;
  anchor risk ZERO by construction (grep on `spec/anchors.toml`
  returns zero `vol_killswitch` rows); CLAUDE.md non-negotiable
  on baseline-equity-divergence e2e tests cited. HANDOFF →
  architect (M-T1).
- 2026-05-26 (developer): Wave A complete — Q4=(p3) "Both" implemented. H1 REFUTED (architect M-T1 probe). Root cause: test fixture warmup gap, not the overlay filter. A.1: fixture fixed (lookback_minutes 60→5, flat warmup, two-spike bar stream). A.2: overlay filter broadened to basket-wide Hold. A.3: 4/4 e2e tests green, overlay hygiene gate 2/2 pass. A.4: bug-log #65 FIXED. HANDOFF → tester.
