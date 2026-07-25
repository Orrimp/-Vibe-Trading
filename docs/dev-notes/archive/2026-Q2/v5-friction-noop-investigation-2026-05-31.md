---
slug: v5-friction-noop-investigation-2026-05-31
status: draft
owner: analyst
updated: 2026-05-31
tags: [v5, latency-slippage-sim, friction, noop, monte-carlo, bootstrap, dual-friction, robustness, carry, integrity, read-only]
related:
  - docs/dev-notes/frame-diagnostic-2026-05-31.md
  - docs/dev-notes/engine-drift-diagnosis-2026-05-30.md
  - spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/feature.md
  - _bmad-output/planning-artifacts/architecture/decisions/0043-simulated-latency-and-slippage.md
  - _bmad-output/planning-artifacts/architecture/decisions/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md
---

# Is the v5 8bps slippage layer a silent NO-OP in the Monte-Carlo bootstrap harness?

> **READ-ONLY investigation** (no cargo, no edits, no git). Mandate: settle whether
> the v5 latency/slippage layer is a *computed-but-unapplied* overlay in the
> bootstrap harness (`run_path`) — the v3-vol-overlay-noop precedent, the project's
> #1 non-negotiable — or whether the harness's `MatchConfig` 2+4 friction is the
> intended/by-design model and v5 is a deliberately separate path.

---

## 0. TL;DR — VERDICT: **BY-DESIGN dual-friction, NOT a no-op bug**

The v5 8bps `slippage_model` layer is **supposed to be inactive** in the
Monte-Carlo bootstrap harness (`montecarlo::run_path`, used by `monte_carlo.rs`
C2 and `param_robustness_sweep.rs` C3/MR/carry). It is **not** a v3-style
no-op bug. There are **two independent friction paths by construction**, and
`run_path` was deliberately written to use only the `MatchConfig` path:

| Path | Code site | Friction applied | v5 `slippage_model` read? |
|---|---|---|---|
| **Bootstrap / robustness harness** (C2, C3, MR, carry) | `crates/backtest/src/scenarios/montecarlo.rs::run_path` | `MatchConfig{slippage_bps:2, taker_fee_bps:4}` → `PaperEngine::step` ONLY (~12 bps round-trip) | **NO** — `latency_slippage_sim` field is passed in but **never read**, and `sim_slippage_cost` is **never called** |
| **Scenario / anchored-backtest path** (determinism.rs t622/t717, the v5 re-emits) | `crates/backtest/src/scenarios/{momentum,pairs,tcn,…}.rs::run` | `MatchConfig{2,4}` from `PaperEngine::step` **PLUS** `sim_slippage_cost(&input.latency_slippage_sim,…)` added to cash | **YES** — `Linear{bps:8}` for synthetic scenarios via `main.rs::build_slippage_model_for_scenario` |

The frame-diagnostic's phrasing — *"the v5 8bps layer appears NOOP in this
harness"* — is **factually correct AND by-design**, not a bug. The word "NOOP"
is the source of the false-alarm: in the bootstrap harness the v5 layer is not
"computed but not applied" (the v3 failure mode); it is **never computed at
all** (the `monte_carlo.rs` / `param_robustness_sweep.rs` drivers hand `run_path`
a `LatencySlippageSimConfig::default()` = noop, and `run_path` doesn't even read
it). That is the critical distinction from the v3 precedent.

This **reconciles cleanly** with the engine-drift diagnosis: v5's `Linear{bps:8}`
moved the determinism.rs SYNTHETIC-scenario SHAs because those tests exercise the
**scenario path** (`momentum::run` etc.), which **does** read `latency_slippage_sim`.
**v5 applies in the scenario path and is absent from the bootstrap path — exactly
as the two-path table predicts.** No contradiction.

**Impact: NOT verdict-affecting.** The momentum/MR FAMILY-UNIFORM-FRAGILE
verdicts stand regardless (the frame-diagnostic's E2 already proved momentum is
fragile even at **0 bps** → the qualitative verdict is friction-insensitive). The
concern is **integrity/documentation + the forthcoming carry verdict's friction
pin**, addressed in §5–§6.

---

## 1. The evidence chain (read-only trace)

### 1.1 Bootstrap path: `run_path` reads `MatchConfig` only, never v5

`crates/backtest/src/scenarios/montecarlo.rs::run_path` (lines 76–316):

- Extracts `slippage_bps = input.slippage_bps` and `taker_fee_bps =
  input.taker_fee_bps` (lines 93–94).
- Builds the engine friction from those two scalars only:
  ```rust
  // montecarlo.rs:111–117
  let match_config = crate::paper::MatchConfig {
      slippage_bps,            // = input.slippage_bps (2)
      taker_fee_bps,           // = input.taker_fee_bps (4)
      maker_fee_bps: 2,
      fill_price_mode: crate::paper::FillPriceMode::BarClose,
  };
  let mut engine = crate::PaperEngine::new(match_config, fill_seed);
  ```
- Applies cost ONLY via `engine.step()` (lines 215, 259). The cash updates
  (lines 232, 263) use `fill.fee.amount()` + `notional_fill` — there is **no
  `sim_slippage_cost` call anywhere in the function body**.
- The `input.latency_slippage_sim` field is **never referenced** in `run_path`.
  (Confirmed by grep: `latency_slippage_sim` appears in `montecarlo.rs` only at
  line 353 — inside the *unit test's* struct literal, set to `::default()`.)

This is **deliberate**, per the module's own contract (montecarlo.rs:20–24):

> *"## R-NR.2 compliance — This module contains NO change to `PaperEngine`,
> `MatchingEngine`, or any scenario `run()`. It is a new thin wrapper that reuses
> the existing engine with a caller-supplied path and strategy."*

`run_path` was authored (ADR-0051 D1 seed-orthogonality work) as a minimal
sibling of `threshold_sweep::run_cell`. Its friction surface is the
`MatchConfig`, full stop. It never opted into the v5 `sim_slippage_cost` plumbing
that the scenario `run()` functions carry.

### 1.2 Both harness drivers hand `run_path` a NOOP v5 config (and 2/4 MatchConfig)

- **C2 driver** `monte_carlo.rs::run_one_path` (lines 801–880) builds:
  ```rust
  slippage_bps: 2,
  taker_fee_bps: 4,
  …
  latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
  ```
- **C3/MR/carry driver** `param_robustness_sweep.rs::run_one_path_with_config`
  (lines 1219–1300) is byte-identical except `slippage_bps`/`taker_fee_bps` are
  now caller-supplied (the **disposable** frame-diagnostic flag, defaults 2/4 —
  flagged for revert in that note's §5). It **also** passes
  `latency_slippage_sim: …::default()`.

`LatencySlippageSimConfig::default()` = `SlippageModel::Linear { bps: 0 }`
(`cli_types.rs:86`; unit test `latency_slippage_sim_config_default_is_noop` at
:181 asserts it). So even if `run_path` *did* read the field, it would be a
hard-zero noop. **Two independent reasons** the v5 layer is inert in the
bootstrap harness: (a) `run_path` doesn't read it, and (b) the drivers pass the
zero default anyway.

### 1.3 Scenario path: `momentum::run` DOES read + apply v5 (the determinism.rs path)

`crates/backtest/src/scenarios/momentum.rs::run` (the path t717
`top10-2023-1h-momentum` exercises via `scenario_body_hex`):

- Same `MatchConfig` from `input.slippage_bps`/`taker_fee_bps` (lines 274–280).
- **PLUS** `sim_slippage_cost` on every fill, added to cash:
  ```rust
  // momentum.rs:387–396 (Buy) and 436–445 (Sell)
  let sim_slip_cost = sim_slippage_cost(
      fill.qty.get(), fill.price.get(), Side::Buy,
      &input.latency_slippage_sim,   // ← reads the v5 config
      &sig.symbol,
  );
  cash -= notional_fill + fill.fee.amount() + sim_slip_cost;
  ```
- `sim_slippage_cost` (`scenarios/sim.rs:52`) dispatches on
  `cfg.slippage_model`: at `Linear{bps:0}` returns `Decimal::ZERO` (noop); at
  `Linear{bps:8}` returns `qty·price·0.0008` (the 8bps friction).

The 8bps value reaches this path via `main.rs::build_slippage_model_for_scenario`
(main.rs:195), which returns `cost::SlippageModel::Linear { bps: 8 }` for
synthetic scenarios (Q-D1=(a), operator-ratified 2026-05-29). **This is the layer
the engine-drift diagnosis identified as moving the t622/t717 synthetic SHAs from
the noop baseline (`fc2e3b4a…`) to the 8bps value (`d2fa7616…`).**

### 1.4 The dispatch helper never reaches the bootstrap bins

`build_slippage_model_for_scenario` is referenced **only** in `main.rs` (8 call
sites: momentum/pairs/tcn/tcnw/ptst/vt/regime/sma scenario arms — main.rs:1359…2054).
Grep confirms **neither** `monte_carlo.rs` **nor** `param_robustness_sweep.rs`
calls it, nor do they call any scenario `run()` (they call `run_path` directly).
So the 8bps value has **no wiring route** into the bootstrap harness — it is
structurally confined to the `main.rs` scenario dispatch. The two paths share the
`PaperEngine`/`MatchConfig` core but diverge entirely on the v5 overlay.

---

## 2. Why this is NOT the v3-vol-overlay-noop failure mode

The CLAUDE.md non-negotiable targets a specific bug shape: *"a no-op overlay where
`scale` is computed but never applied."* The v3 vol-forecaster computed a sizing
`scale` and then dropped it on the floor inside the **same** decision path that
was supposed to consume it. The fix/gate was an e2e divergence test.

The v5/bootstrap situation is **categorically different**:

| | v3-vol-overlay-noop (a BUG) | v5-in-bootstrap (BY-DESIGN) |
|---|---|---|
| Is the value computed? | Yes — `scale` computed every bar | **No** — `run_path` never computes a v5 cost; driver passes noop config |
| Is it expected to apply *here*? | **Yes** — it was the whole point of that path | **No** — `run_path`'s friction contract is `MatchConfig`; v5 lives in the scenario `run()` path |
| Silent divergence from intent? | Yes — intent was "apply scale," code didn't | No — intent (montecarlo.rs module doc + ADR-0051 D1 R-NR.2) is "reuse engine, no scenario-`run` plumbing" |
| Detection gate | e2e equity-divergence test | N/A — no overlay is claimed for this path |

The v3 precedent requires a divergence gate **for overlays that are supposed to
apply on a given path**. The v5 layer makes **no such claim** on the bootstrap
path. There is therefore no missing gate here — though §5 recommends a
*documentation* fix so a future reader doesn't re-trip this alarm.

**Caveat (honest):** the *absence* of an explicit "the robustness harness uses
MatchConfig friction, not the v5 model" statement in any ADR is itself the gap
that made this question necessary. The integrity issue is **under-documentation**,
not mis-wiring.

---

## 3. Reconciliation with the engine-drift diagnosis (no contradiction)

The engine-drift diagnosis (2026-05-30) stated v5's 0→8bps change moved the
determinism.rs SYNTHETIC-scenario outputs. That is **100% consistent** with this
finding:

- determinism.rs t622/t717 hash the **scenario path** (`momentum::run`,
  `sma_composed_run`, etc.) — which **reads** `latency_slippage_sim` and gets
  `Linear{bps:8}` from `build_slippage_model_for_scenario`. → v5 **applies** here.
- The bootstrap harness (`run_path`) does **not** read it. → v5 is **absent** here.

So the answer to the brief's pointed question — *"does v5 apply in the scenario
path but NOT the bootstrap path?"* — is **YES, exactly that.** The two diagnoses
describe the two ends of the same two-path architecture. The engine-drift note
even hinted at this (its §"momentum g=0↔C2 probe passed … tests equity
DIVERGENCE … not anchored absolute values"), but did not explicitly state that
`run_path` ignores the v5 field. This note closes that gap.

---

## 4. Is the MatchConfig 2+4 the *intended* robustness friction?

**Yes — by current construction, and it is a defensible (if undocumented)
choice.** Observations:

1. The C2/C3/MR anchored robustness reports (anchor #86 momentum-2023, #87 MR)
   were all produced under `run_path` → `MatchConfig{2,4}`. The
   FAMILY-UNIFORM-FRAGILE verdicts are **defined against 2+4 (~12 bps
   round-trip)** friction. That is the de-facto robustness friction of record.
2. `MatchConfig::default()` is also `{slippage_bps:2, taker_fee_bps:4}`
   (paper.rs:31–39) — so 2+4 is the engine's own baseline, predating v5. The
   robustness harness inherited the pre-v5 paper-engine friction and never
   migrated to the v5 model.
3. **Mismatch worth flagging:** the v5 *canonical* friction is **8 bps slippage**
   (`Linear{bps:8}`, the ADR-0045 D1 pin) applied **on top of** MatchConfig in the
   scenario path. So the scenario path runs ~2 (MatchConfig slip) + 8 (v5 slip) +
   4 (taker) per side, whereas the bootstrap path runs 2 + 4 per side. The
   bootstrap harness is therefore running at **lower friction** than the canonical
   v5 scenario backtests. This is the substantive (non-bug) finding: the two paths
   use **different friction magnitudes**, and neither ADR nor the robustness
   feature docs state which is "the" robustness friction.

Whether 2+4 is *adequate* (vs. the heavier v5 canonical) is a judgement call. For
a **fragility/falsification** harness, the conservative choice would be the
**higher** friction (the v5 8bps canonical), since more friction makes the bar
harder and the fragile verdict more robust. The current harness uses the *lower*
friction — which is the *more generous* case — and momentum/MR still fail. So the
direction of the mismatch does **not** threaten the fragile verdicts (a strategy
that fails at low friction fails harder at high friction). It would only matter if
a future strategy **passed** at 2+4 and we needed to know it also passes at the
canonical 8bps.

---

## 5. Impact assessment

### 5.1 Does it flip momentum/MR verdicts? **NO.**

- The frame-diagnostic E2 ran momentum at **0 bps** (zeroing the MatchConfig 2+4)
  and it was **STILL FAMILY-UNIFORM-FRAGILE** (p5 < 0 and P(Sharpe>1)=0% in all 6
  cells). The qualitative verdict is **friction-insensitive across [0, 12] bps
  round-trip**. Adding the v5 8bps (→ ~28 bps round-trip) would only make momentum
  **worse**, never lift it past the bar. So the missing v5 layer cannot have
  *masked* a passing momentum/MR result.
- MR (anchor #87) is the inverse signal on the same universe/harness; same logic
  applies.

**Conclusion:** the friction-model choice is **not verdict-affecting** for the two
shipped fragile verdicts. The integrity concern is real but the *decisions* taken
on those verdicts are safe.

### 5.2 The carry verdict (forthcoming) — the one place to be deliberate

Carry is the strategy where this matters, for two reasons:
1. Carry's thesis is a **structurally different (funding/basis) return source**
   that the frame-diagnostic argued *might* clear the bar where price-direction
   strategies don't. If carry comes back **marginal** (unlike momentum/MR's
   decisive failure), the friction magnitude could be **pivotal** to a
   PASS/FRAGILE call.
2. Carry trades less (lower turnover) → it is **less** friction-sensitive in
   absolute terms, but a marginal verdict is exactly the regime where the 2+4
   vs. 8+4 gap could move a borderline Sharpe across the promotion threshold.

**Recommendation for the carry build** (hand to the carry architect/analyst — do
NOT edit `spec/carry-strategy/` here): **pin the carry robustness friction
explicitly and consciously.** Two options:

- **(a) Pin carry to the v5 canonical friction (Recommended — durable + conservative).**
  Make `run_path` (or the carry driver) use the canonical 8 bps v5 model (or at
  minimum document that carry is judged at the heavier canonical friction). This
  makes the carry bar the *hardest fair* bar and forecloses a future "but you
  tested it at low friction" challenge. Cost: requires deciding whether to wire v5
  into `run_path` (a real change with anchor implications) OR raising the
  MatchConfig taker/slippage to match. This is the long-term-correct choice if
  carry is ever a promotion candidate.
- **(b) Keep 2+4 for cross-strategy comparability (fallback — cheaper, if budget
  tightens).** Judge carry at the *same* 2+4 the momentum/MR anchors used, so the
  three strategies are compared apples-to-apples on identical friction. Cheaper
  (zero engine change), and defensible *as a comparison*, but leaves carry's
  verdict exposed to the "lower-than-canonical friction" critique if it passes
  marginally. If chosen, document explicitly that the carry verdict is at 2+4
  (~12 bps round-trip), NOT the v5 8bps canonical.

The carry team should make this an explicit, recorded decision **before** running
the carry sweep — not inherit 2+4 silently the way momentum/MR did.

---

## 6. Recommended documentation fix (no code change required for the verdict)

The verdict is "by-design," so **no bug fix is owed**. But to prevent this
false-alarm from recurring and to honour the "no silent divergence" + CLAUDE.md
overlay-integrity spirit, one cheap documentation action is warranted (route to
architect; do NOT execute here):

- **Add a "Friction model: two paths" note** to the robustness-harness
  documentation (the C2/C3 feature docs and/or an ADR-0051 amendment, NOT the
  carry ADR which is locked-in-flight): state plainly that
  `montecarlo::run_path` applies **MatchConfig friction only** (2 bps slip + 4 bps
  taker per side), and that the v5 `slippage_model` / `sim_slippage_cost` layer is
  **scenario-path-only by design** and inert in the bootstrap harness. Cite this
  note. This converts an implicit architectural fact into a stated contract so the
  next reader (and the next diagnostic) doesn't have to re-derive it.

This is a **paperwork** action (mirrors the engine-drift "PAPERWORK" verdict
pattern), and it must respect the byte-immutability of anchored report files
(CLAUDE.md non-negotiable) — i.e., touch feature/ADR docs, never the anchored
`reports/*.md`.

---

## 7. Assumptions & limits (challengeable)

1. **Read-only, static trace.** No cargo run confirmed the runtime behaviour; the
   verdict rests on reading `run_path` (no `sim_slippage_cost` call, no
   `latency_slippage_sim` read), the two drivers (noop config passed), and the
   grep that `build_slippage_model_for_scenario` never reaches the MC bins. A
   1-line runtime confirmation (instrument `run_path` to assert
   `input.latency_slippage_sim.is_noop()` and that no sqrt branch executes) would
   make it airtight, but the static evidence is unambiguous — all three
   independent checks agree.
2. **"Not verdict-affecting" leans on the E2 0bps result.** That result is from
   the frame-diagnostic's disposable-flag run (N=200, seed 0xC0FFEE). It is the
   correct generalization (fragile at 0 ⇒ fragile at any friction ≥ 0), so the
   conclusion is robust, but it inherits E2's N=200 resolution.
3. **The MatchConfig "slippage_bps:2" is itself a spread/slippage term** applied
   inside `PaperEngine::step` (paper.rs:83–84), distinct from the v5
   `sim_slippage_cost`. So the bootstrap harness is **not** frictionless — it has
   2 bps slip + 4 bps taker per side. "v5 NOOP here" does **not** mean "no friction
   here." This is the precise point the word "NOOP" obscured in the frame-diagnostic.
4. **Carry recommendation is a steer, not a decision.** The friction pin for carry
   is an operator/carry-architect call; this note only flags it as the one place
   the dual-friction model could matter and frames the durable-vs-cheap options.
5. **Scope honoured:** did not read or touch `spec/carry-strategy/` or
   `_bmad-output/planning-artifacts/architecture/decisions/0051-*.md` (carry architect editing concurrently). The
   ADR-0051 D1 reference here is from the `montecarlo.rs` module doc comment only.

---

## Changelog

- 2026-05-31 (analyst, v5-friction-noop-investigation): READ-ONLY trace settling
  whether the v5 8bps slippage layer is a silent no-op in the Monte-Carlo
  bootstrap harness. **VERDICT: BY-DESIGN dual-friction, NOT a no-op bug.**
  `montecarlo::run_path` (C2/C3/MR/carry) applies `MatchConfig{2,4}` via
  `PaperEngine::step` ONLY and never reads `input.latency_slippage_sim` nor calls
  `sim_slippage_cost`; both drivers pass `LatencySlippageSimConfig::default()`
  (noop). The v5 `Linear{bps:8}` layer reaches ONLY the scenario path
  (`momentum::run` etc., via `main.rs::build_slippage_model_for_scenario`), which
  is what moved the determinism.rs t622/t717 synthetic SHAs — fully consistent
  with the engine-drift diagnosis (v5 applies in scenario path, absent in
  bootstrap path). Distinct from the v3-vol-overlay-noop bug: v5 is never
  *computed* on the bootstrap path (not computed-but-unapplied). **Impact:
  NOT verdict-affecting** — E2 showed momentum fragile even at 0bps, so the
  qualitative verdict is friction-insensitive; the missing v5 layer cannot have
  masked a passing result. Substantive finding: the two paths run at DIFFERENT
  friction (bootstrap 2+4 ≈12bps RT; scenario 2+8+4 canonical), and neither ADR
  states which is "the" robustness friction. Recommendations: (a) carry build must
  PIN its robustness friction explicitly (Recommended: v5 canonical 8bps for the
  hardest-fair bar; fallback: 2+4 for cross-strategy comparability, documented);
  (b) add a "two friction paths" note to the robustness-harness docs / ADR-0051
  amendment (paperwork, not a bug fix; respect anchored-report immutability). Did
  NOT touch spec/carry-strategy/ or adr/0051-*.md (concurrent carry edit).
