---
slug: advisor-ensemble
status: shipped
owner: tester
updated: 2026-06-22
---

# F8 — Strategy-mix ensemble candidates for the single-coin advisor

> **One-line framing:** the bake-off can now *also* consider a small, fixed,
> pre-registered set of **principled mixes** of the existing single strategies —
> each entered as a candidate that must EARN its crown through the **same
> robustness gate + the same buy-and-hold benchmark** every single strategy
> already faces. This is **not** "mixes make you more money." It is "we offer a
> few honest mixes and measure them like everything else."

## Why

The 2026-06-19 product redefinition (see [`../product.md`](../../product.md))
named v0.2 explicitly: *"…the selected strategy or even a mix of strategies and
even together with LLMs or other Machine learning."* The MVP (F1–F6 +
dynamic-data + budget sizing) ships **one** strategy. F8 is the "mix" capability.

It is also the single sharpest **honesty hazard** in the whole product. The
concluded research verdict (2026-06-08, retained in `product.md § Why this is
honest`) is that *no active strategy beat passive buy-and-hold net of cost under
a frozen block-bootstrap robustness rule.* **Ensembles are the easiest way to
accidentally manufacture an in-sample winner**: combining N strategies multiplies
researcher degrees of freedom, and an unconstrained search over weights/votes
will *always* surface something that beat buy-and-hold on the realized path. If
F8 is built naively it silently converts the product from "measured robustness,
not asserted alpha" into "data-mining with extra steps" — destroying the one
thing that makes the bake-off trustworthy.

So the design problem is not "how do we combine strategies" (mechanically easy)
— it is **"how do we let mixes compete without letting them cheat."** The answer
runs through three locks: (1) a **bounded, pre-registered** candidate set (a
fixed handful, declared in code, never a runtime weight search); (2) the **same
ranking comparator** (`rank_candidates`) with the **same Fragile-ineligible
gate** that already governs the singles; (3) the **same buy-and-hold benchmark
arm** that is always present and always honest — `BenchmarkWins` and `AllFragile`
must remain reachable outcomes for an ensemble-bearing field.

## The honest F8 definition (load-bearing — 4 sentences)

1. An **ensemble** in this product is a **deterministic combination of the
   existing single strategies' BUY/SELL signals into one composite signal
   stream** that trades a single position on the same `(coin, window)` — it is a
   new *candidate arm*, not a new asset class and not a new alpha source.
2. The bake-off enters a **small, fixed, code-declared set** of such ensembles
   (proposed below: **two** candidates) alongside the singles + buy-and-hold;
   there is **no runtime search over weights, thresholds, or membership** — the
   set is pre-registered exactly like a falsifier, so it cannot be tuned to win.
3. Every ensemble is ranked by the **identical** comparator the singles face
   (Fragile-ineligible → Sharpe → return → drawdown → id) against the **identical
   buy-and-hold benchmark**, and its robustness flag is computed on its **own
   realized equity curve** — an ensemble that wins one path but is FRAGILE under
   resampling is shown but **cannot** be crowned (unless everything is Fragile).
4. If no ensemble beats the single-best pick **and** buy-and-hold under the gate,
   the recommendation surface **says so honestly** — F8 must not change the
   reachability of `BenchmarkWins` / `AllFragile`; it only widens the field.

## Requirements

### R1 — Ensemble is a *signal-combination* candidate (the WHAT)

**(Recommended combination method: signal vote.)** Of the three options the
brief names —

- **Signal vote** — combine the BUY/SELL signals of N member strategies into one
  composite signal (majority / unanimous / weighted), trading a single position.
- **Capital split** — allocate slices of the €200 across N strategies, each
  trading its own slice; equity is the sum of sleeves.
- **Meta-selector** — a regime label picks which single strategy is active.

— the recommended v1 method is the **signal vote**, for four reasons grounded in
the code as it exists today:

1. **It reuses the real seam.** `StrategyRegistry::on_bar` already fans a bar out
   to all registered strategies and returns `Vec<Signal>` (see
   `crates/strategy/src/registry.rs`). A vote is a thin, pure, deterministic
   **arbitration function** over that vector — `Vec<Signal> → Option<Signal>` —
   not a new engine. This is the smallest honest surface.
2. **It produces one position, one equity curve, one Sharpe, one robustness
   flag** — so it slots into `CandidateResult` / `rank_candidates` /
   `RobustnessFlag` with **zero changes to the ranking contract**. Capital-split
   produces a summed-sleeve equity whose robustness semantics are murkier (each
   sleeve is tiny; block-bootstrap on the blended curve is harder to defend);
   meta-selector needs a regime label the project does not honestly have (see R2).
3. **It is honestly describable as a plan (F6).** "BUY when ≥ 2 of {SMA, MACD,
   RSI} agree; SELL when the majority flips" is a legible conditional rule, the
   same shape `PlanRuleShape` already encodes. Capital-split's plan is "run three
   plans at once on a third of the budget each" — harder to render honestly and
   tripling the F6 surface.
4. **It is the cheapest to gate against overfit.** One composite curve → one
   robustness read → one comparator row. The data-mining risk is contained to
   "how many vote rules do we pre-register," which R3 bounds to **two**.

**Capital-split is the named v0.3 fallback** if the operator wants
diversification-of-sleeves semantics rather than signal-consensus — it is
genuinely additive on top of the vote (the registry already supports N active
strategies; only the sizing/equity-aggregation seam is new). It is **not**
recommended for v1 because its robustness story is weaker and it roughly triples
the F4/F5/F6 integration surface.

> **NON-GOAL (R1):** the ensemble does **not** introduce a weighted-vote *search*.
> If a weighted vote is ever offered, the weights are **fixed constants declared
> in code** (e.g. equal weight), never fit to the window. A "find the best
> weights" loop is data-mining and is explicitly out of scope at every version.

### R2 — LLM / ML participation: **narration-only in v1 (do NOT wire as a signal)**

**Verified against the code, not the spec prose:**

- **The LLM is genuinely narration / tool-use plumbing.** `crates/llm`'s trait is
  `LlmProvider::complete(ChatRequest) -> ChatResponse` returning text + tool-use
  blocks (`crates/llm/src/trait_def.rs`). **Neither `crates/strategy` nor
  `crates/backtest` imports `crates/llm`** (verified: grep finds no `llm::` in
  either). There is **no runnable path today where an LLM emits a `Signal` or a
  `Direction` into a backtest.** The "LLM-as-analyst overlay" arm named in
  `product.md` is **aspirational for the bake-off — it does not exist as a
  dispatchable `run_scenario` arm.** Wiring it is a multi-week feature (provider
  in the hot loop, record/replay determinism for the bake-off seed, cost budget
  per arm), not an F8 sub-task.
- **The ML that exists is the retired forecaster family.**
  `crates/forecast::ForecastProvider::forecast()` is the ML signal trait, and a
  real signal-modulation overlay exists (`forecast::overlay::combine`, wired as
  the `v2.5.tcn` `run_scenario` arm). **But** the only `ForecastProvider` impls
  are `MockForecaster` (tests) and the **retired** TCN / PatchTST / GARCH-σ /
  LLM-forecaster chains — all concluded *not beating passive* (product.md
  § Strategy library; `MEMORY.md` "only PIT data is worth it"). Per the brief and
  the CLAUDE.md/MEMORY non-negotiable, **F8 must NOT resurrect a retired ML
  forecaster as if it were new alpha.**

**Therefore in v1 the LLM/ML role is exactly what it honestly is today:** the
"why this one" **narration** over the crowned candidate (already the sanctioned
LLM role, F9 scope), and **nothing in the ranking path.** The ensemble's members
are the four **deterministic rule engines** that already run cleanly in the field
(`v0.sma`, `v0.5.macd`, `v0.5.rsi`, `v0.5.bbands`). The buy-and-hold arm is the
benchmark, not a vote member.

> **NON-GOAL (R2):** F8 does NOT add an LLM or an ML model as a vote member or a
> ranking input. The LLM stays support-layer (narration). Any "LLM-as-analyst
> confirms the vote" capability is a **separate, later** feature
> (`v26-bakeoff-llm-arbiter`, already parked in `backlog.md § Gated on the parked
> v2 LLM strategy`) that must clear its own determinism + cost-budget design and
> its own robustness gate — it is **not** smuggled in under F8.

### R3 — The bounded, pre-registered candidate set (the WHICH)

The ensemble field is a **fixed, code-declared constant** — the ensemble analogue
of `BakeoffConfig::default_field()`. Proposed v1 set: **exactly two** ensembles,
chosen because each is a *named, defensible consensus rule* rather than a point in
a tunable space:

- **`v0.8.vote.majority`** — **equal-weight majority vote of the three trend/
  reversion rule engines** (`v0.5.macd`, `v0.5.rsi`, `v0.5.bbands`): hold LONG
  when **≥ 2 of 3** members are currently LONG; flat otherwise. (SMA is excluded
  from this one to keep it an odd-membered, tie-free vote of the three v0.5
  composed engines; see OQ-ENS-3 — membership is itself a pre-registration
  choice, not a free parameter.)
- **`v0.8.vote.unanimous`** — **unanimous vote of all four** rule engines
  (`v0.sma`, `v0.5.macd`, `v0.5.rsi`, `v0.5.bbands`): hold LONG only when **all
  four** agree LONG; flat otherwise. This is the maximally-conservative consensus
  arm — it trades rarely and exists to test whether *agreement* (not any single
  rule) carries signal.

Rationale for **two**: it is the smallest set that contrasts the two honest
consensus hypotheses (*loose majority* vs *strict unanimity*) without becoming a
search. Two ensembles + 4 singles + buy-and-hold = a **7-arm field** — a
leaderboard a retail user can still read, and a multiple-comparisons burden small
enough that the robustness gate is not overwhelmed (flag for architect: see
OQ-ARCH-2 on whether the gate needs a multiple-comparisons hardening even at 7
arms — my read is **not yet**, but it must be re-evaluated, never assumed).

> **NON-GOAL (R3):** the candidate set is **frozen in code**. There is **no UI
> control, no config knob, and no runtime loop** that adds, removes, reweights, or
> re-thresholds ensemble candidates. Growing the set (e.g. adding a weighted vote)
> is a deliberate spec change with its own review — exactly like adding a
> falsifier. A user-facing "build your own mix" surface is explicitly out of scope
> (it would reintroduce unbounded researcher DoF at the worst possible layer: the
> end user).

### R4 — Ranking + robustness: same gate, no exemption (the credibility lock)

- **R4.1** Each ensemble produces **one realized equity curve** from the matching
  engine, exactly like a single. `derive_candidate_kpis` (Sharpe/Sortino/Calmar +
  return/drawdown/trade-count) applies **unchanged**.
- **R4.2** Each ensemble is a `CandidateResult` ranked by the **unchanged**
  `rank_candidates` comparator. `is_benchmark = false` for ensembles. The crown is
  whatever the comparator says — an ensemble wins **only** if it is non-Fragile
  **and** out-Sharpes every other eligible arm including the singles and
  buy-and-hold.
- **R4.3** **Robustness is computed on the ensemble's own curve.** This is the
  non-negotiable anti-overfit lock. **Today the gate is inert** —
  `RobustnessMode` has only a `Skip` variant, so every live bake-off flag is
  `None` and *nothing is ever Fragile* (verified in
  `crates/backtest/src/bakeoff/mod.rs`). For ensembles this is the **single most
  important architect decision**: shipping ensembles **without** a real
  block-bootstrap compute mode means the easiest-to-overfit candidates run through
  a gate that **cannot reject them.** **F8 should land together with a real
  robustness compute mode** (`RobustnessMode::Bootstrap { paths, .. }`) so the
  Fragile-ineligible branch actually fires — see OQ-ARCH-2. (The classifier
  itself, `robustness::classify_verdict`, is already correct and frozen; only the
  per-candidate *compute-and-feed* path is missing.)
- **R4.4** `RecommendationOutcome::BenchmarkWins` and `::AllFragile` **remain
  reachable** for an ensemble-bearing field. A regression test must assert that a
  field where buy-and-hold has the top eligible Sharpe still yields `BenchmarkWins`
  even when ensembles are present, and that an all-Fragile field (singles +
  ensembles) still yields `AllFragile`.

> **NON-GOAL (R4):** ensembles get **no** ranking bonus, no tie-break preference,
> no "diversification credit," and no exemption from the Fragile gate. They are
> peers of the singles in every respect except that they are composite.

### R5 — Forward-run (F5b) + plan (F6): make the ensemble runnable + describable

An ensemble must also be (a) **paper-tradable forward** and (b) **describable as a
plan**, or it is a leaderboard-only curiosity that the journey can crown but never
run — which would be dishonest (the user is told "this is best" but cannot watch
it). Both touch hardcoded-id-string dispatch seams that currently only know the 6
singles:

- **R5.1 (forward-run, F5b).** `agent::runtime::build_registry_for(id)` maps an id
  to **exactly one** `Box<dyn Strategy>` (verified). An ensemble id has **no**
  single `Strategy` to register — it needs either (i) a new `EnsembleStrategy`
  adapter that *implements `Strategy`* by internally holding its members and
  arbitrating their signals in its own `on_bar` (cleanest — it becomes a normal
  registry citizen and the whole F4/F5 forward path works unchanged), or (ii) a
  registry that drives multiple members + an external arbiter (more invasive,
  changes the supervisor loop). **Flag for architect (OQ-ARCH-1):** the
  `EnsembleStrategy`-implements-`Strategy` adapter (i) is almost certainly the
  durable choice — it makes the vote a first-class strategy everywhere
  (`run_scenario` dispatch, `build_registry_for`, `StrategyRegistry`) with one
  arbitration primitive, rather than special-casing ensembles at three call sites.
- **R5.2 (plan, F6).** `PlanRuleShape` is a **closed enum** with no ensemble
  variant (`crates/strategy/src/plan.rs`), and `ComposedStrategy::describe_plan`
  maps id-strings to fixed rule shapes. An ensemble needs a **new
  `PlanRuleShape::Ensemble { method, members }`** variant (structured data only —
  the `ui` owns the copy, per the ADR-0059 Recommendation-not-a-String precedent)
  so the plan can honestly say "BUY when ≥ 2 of {MACD, RSI, BBands} are LONG; the
  current vote is 1/3 → FLAT." The `EnsembleStrategy` adapter (R5.1.i) would carry
  the `PlanDescribe` impl. **Flag for architect (OQ-ARCH-3):** confirm the closed
  `PlanRuleShape` enum is the right place to extend and that the `ui`'s exhaustive
  match is updated in lockstep.

> **NON-GOAL (R5):** F8 does NOT change the F5 paper-loop supervisor contract or
> the F6 read-only `PlanDescribe` non-mutation contract. The ensemble adapter must
> satisfy both as they stand — it is a new *member* of those contracts, not an
> amendment to them.

## Backtest Scenarios

_analyst + architect fill this using the backtest/scenario template once the
combination method + candidate set are operator-ratified._ Provisional shape:

- **No new anchored scenario.** Like F1–F6, F8 *runs* the already-anchored
  strategies (as vote members) but introduces **no new anchored backtest report**;
  `scripts/verify_anchors.sh` must stay **119/119** before and after. The ensemble
  arms run on the same dynamic/pinned bars the bake-off already resolves.
- **Day-1 baseline-equity-divergence e2e (CLAUDE.md non-negotiable):** the
  `EnsembleStrategy` adapter **is a strategy whose decision variable is
  non-trivial** — so the non-negotiable applies. The required gate: an e2e test
  asserting the **majority-vote ensemble's equity curve diverges from each member
  single's equity curve** (and from the un-voted baseline) by ≥ 1 bp when the
  members disagree on at least one bar. This catches the analogue of the v3-vol
  no-op bug (a vote that is computed but silently passes through one member's
  signal). Pattern reference:
  `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`.

## Design

_Architect, 2026-06-21. Full rationale + alternatives in
[ADR-0063](../../../_bmad-output/planning-artifacts/architecture/decisions/0063-ensemble-vote-seam-and-robustness-gate-activation.md).
Operator forks resolved: OQ-OP-1 = signal-vote; OQ-OP-2 = LLM/ML narration-only;
OQ-OP-3 = exactly two pre-registered votes. The three architect OQs resolve as
follows._

### OQ-ARCH-1 — the `EnsembleStrategy` seam (ONE primitive, three seams)

`strategy::EnsembleStrategy` is a concrete struct that **implements the FROZEN
`Strategy` trait** (ADR-0005 not modified) — the `RegimeDispatcher`
(`crates/strategy/src/regime_dispatcher.rs`) precedent generalised to N
homogeneous members + a consensus arbiter. Because it *is* a `Strategy`, it is a
normal registry citizen reached from ONE id→members mapping at all three call
sites with zero special-casing:

- **Members are reused, not reimplemented.** A shared
  `build_member(id) -> anyhow::Result<Box<dyn Strategy>>` constructs each member
  through the **existing** per-id construction (`SmaCrossover` for `v0.sma`;
  `ComposedStrategy` loaded from `config/strategies/{btc_macd_trend,
  btc_rsi_reversion,btc_bbands_mean_revert}.toml` for the v0.5 arms — the SAME
  TOMLs `build_registry_for` already loads). One source of member truth.
- **The arbiter is a pure free fn** `arbitrate(method, &stances) -> Stance`
  (`Stance ∈ {Long, Flat}`). The ensemble tracks each member's current LONG/FLAT
  stance (edge-triggered Buy → Long, Sell → Flat — the same stance the singles'
  equity expresses), recomputes consensus per bar, and emits Buy/Sell only on the
  **ensemble's own** stance transition (edge-triggered, matching
  `ComposedStrategy`). Methods frozen in code:
  `VoteMethod::Majority { k: 2, n: 3 }` (Long iff `long_count ≥ k`) and
  `VoteMethod::Unanimous { n: 4 }` (Long iff `long_count == n`).
- **Warmup / abstention rule (the honest count).** A member that has not warmed
  up (`ComposedStrategy::last_rule_value() == None`, SMA `current() == None`)
  **ABSTAINS** — it is counted in NEITHER `long_count` NOR the denominator. The
  ensemble is FLAT (emits nothing, abstains itself) **until its needed quorum is
  warm** (`Majority` needs ≥ k warmed; `Unanimous` needs all 4). Counting an
  un-warmed member as FLAT is REJECTED: it manufactures false majorities from
  early-warming members and hides consensus. Determinism: warmup is a pure
  function of the (identical) bar sequence each member sees.
- `quantity_scale` stays the default `1.0` (a signal combiner, not a sizing
  modifier; sizing is F4's `budget_cap`, untouched).

The two ids are opt-in field entries — `BakeoffConfig::default_field()` is
unchanged; a new `default_ensemble_field()` carries the two votes, composed only
by the advisor caller.

### OQ-ARCH-2 — activate the gate (`RobustnessMode::Bootstrap`)

Add `RobustnessMode::Bootstrap { paths: usize, seed: u64 }` (default stays
`Skip`). The missing compute-and-feed path, per candidate, after its equity curve
exists:

1. equity → log-returns (reuse the `derive_candidate_kpis` mapping).
2. **block length** = the EXISTING `data::synth::block_length::
   politis_white_block_length(&returns)` (Politis–White PWSD — NOT a magic
   constant; the project's established choice, ADR-0051).
3. **resample** `paths` = **1000** moving-block draws; determinism via the FROZEN
   ADR-0051 D1 sub-seed rule `path_seed_j = master.wrapping_add(j*0x9E3779B9)`,
   one `ChaCha20Rng::seed_from_u64(path_seed_j)` per path; `master` DERIVED from
   the bake-off seed (+ a fixed per-candidate salt so two candidates do not share
   a draw) ⇒ same bake-off seed → same flags.
4. per path → `PathMetrics`; reduce via the EXISTING
   `DistributionSummary::from_path_metrics`; classify via the FROZEN
   `classify_verdict` (bands NOT touched). Populate `CandidateResult.robustness`
   that `rank_candidates` already reads.

**Multiple-comparisons: re-evaluated, decision NOT-YET at a 7-arm field.** The
gate is an absolute per-candidate band rule (p5/p50 Sharpe, prob_loss, …), NOT a
"best-of-K" significance test; widening the field does not inflate a
per-candidate false-positive rate the way K significance tests would. The
pre-registration lock (R3) removes the search DoF that drives the classic hazard.
**Trigger recorded:** a candidate set that grows with data, or any
selection-over-configs step, REQUIRES its own ADR (Bonferroni/Šidák band
tightening or a White Reality-Check / SPA test).

### OQ-ARCH-3 — `PlanRuleShape::Ensemble { method, members }`

Extend the closed `PlanRuleShape` enum with one variant carrying **structured
data only**: `Ensemble { method: PlanVoteMethod, members: Vec<PlanRuleShape> }`
(`PlanVoteMethod` = a new closed `Majority { k, n } | Unanimous { n }` enum, NOT a
string). The `EnsembleStrategy` carries the `PlanDescribe` impl (ADR-0062 sibling
trait, non-mutating read of member stances + arbiter). The `ui` exhaustive match
gains an `Ensemble` arm rendering honest copy that **names the method + members**
(e.g. "Holds when ≥ 2 of {MACD trend, RSI reversion, Bollinger reversion} agree;
goes flat when the majority flips" + the live tally), with `members` carrying each
member's own rule shape. `agent`↔`ui` mirror in lockstep; the
no-`String`-crosses-the-seam discipline (ADR-0059 / ADR-0062) is kept end to end;
`cargo tree -p ui` unchanged.

### Anchor safety (119/119 by construction) — see ADR-0063 § D5

`RobustnessMode::default()` STAYS `Skip` ⇒ every anchored CLI path keeps `None`
flags ⇒ ranking byte-unchanged. ONLY the advisor `spawn_bakeoff` path (which
writes NO report — ADR-0059 § D3 `write_report = false`) opts into `Bootstrap`,
computing in-memory flags ONLY. The two ensemble arms are opt-in field entries
(the `v0.buyhold` / dynamic-cache / `budget_cap` additive precedent);
`default_field()` and the existing single-id `run_scenario` arms are
byte-untouched. No anchored report is opened/edited/re-emitted; no `anchors.toml`
SHA or `REVISION.toml` changes; no new anchored scenario. Run
`scripts/verify_anchors.sh` before the first seam and after the last — non-119 is
a STOP-and-route-back.

### Day-1 divergence gates (CLAUDE.md non-negotiable) — see ADR-0063 § D7

Two FAIL-before / PASS-after e2e gates ship WITH the code:
**(a)** `ensemble_vote_divergence_end_to_end.rs` — the majority ensemble's equity
diverges from EACH member's curve by ≥ 1 bp on a disagreement window (catches a
silent passthrough = the v3-vol no-op analogue); **(b)**
`robustness_bootstrap_bites.rs` — an overfit candidate is flagged `Fragile` +
loses the crown to a robust single, AND a robust candidate is NOT flagged (no
false-positive) — the anti-`Skip`-regression gate. Plus the F2 reachability
regression (BenchmarkWins / AllFragile reachable with ensembles present).

## Implementation

Implemented 2026-06-21 by developer (claude-sonnet-4-6). All backend tasks
D-T1 through D-T6.3 completed; D-T3.4, D-T5.3, D-T5.4, and UI tasks remain for
ui-designer and tester.

### New files

- `crates/strategy/src/ensemble.rs` — `EnsembleStrategy`, `VoteMethod`,
  `MemberStance`, `EnsembleBuildError`, pure `arbitrate()`, `build_member()`,
  `build_ensemble()` factory, `PlanDescribe` impl. Thiserror used (anyhow
  removed per ADR-0041 D1). 950 lines including 15 unit tests.
- `crates/backtest/src/bakeoff/bootstrap.rs` — `compute_robustness_flag()`,
  `derive_master_seed()`, `SALT_TABLE`, `GOLDEN_GAMMA`. Pure function of
  (`&[Decimal]`, `paths`, `master_seed`) → `RobustnessFlag`. 370 lines
  including 9 unit tests.
- `crates/strategy/tests/ensemble_vote_divergence_end_to_end.rs` — 12 e2e
  tests (D-T5.1 gate): SMA-based divergence, warmup abstention, determinism,
  factory smoke, arbitrate pure tests.
- `crates/backtest/tests/robustness_bootstrap_bites.rs` — 14 integration tests
  (D-T5.2 gate): fragile/not-fragile classification, determinism, skip→None,
  bootstrap→populated flags, additive field contract.

### Modified files

- `crates/strategy/src/lib.rs` — exports `EnsembleBuildError`, `EnsembleStrategy`,
  `MemberStance`, `VoteMethod`, `arbitrate`, `build_ensemble`, `build_member`.
- `crates/strategy/src/plan.rs` — added `PlanVoteMethod` enum + `PlanRuleShape::Ensemble`
  variant (structured data only, no copy string).
- `crates/strategy/Cargo.toml` — registered `[[test]] ensemble_vote_divergence_end_to_end`.
- `crates/backtest/src/bakeoff/mod.rs` — `RobustnessMode::Bootstrap { paths, seed }`
  variant; `BakeoffConfig::default_ensemble_field()` method; `Bootstrap` arm in
  `run_bakeoff` loop; candidate_index enumeration; `pub mod bootstrap` + re-exports.
- `crates/backtest/src/engine.rs` — `"v0.8.vote.majority" | "v0.8.vote.unanimous"`
  dispatch arm using `strategy::build_ensemble`.
- `crates/backtest/Cargo.toml` — registered `[[test]] robustness_bootstrap_bites`.
- `crates/agent/src/config.rs` — `PlanRuleKind::Ensemble { method, member_count }`,
  `PlanVoteMethod` enum.
- `crates/agent/src/plan.rs` — `PlanRuleShape::Ensemble` → `PlanRuleKind::Ensemble`
  mapping in `map_rule_shape`; `build_forward_plan_from_registry` ensemble arms.
- `crates/agent/src/runtime.rs` — `build_registry_for` ensemble arms.

### Test results (verified locally)
- `cargo test -p strategy`: 195+ tests pass, 0 failures
- `cargo test -p strategy --test ensemble_vote_divergence_end_to_end`: 12/12 pass
- `cargo test -p backtest --test robustness_bootstrap_bites`: 14/14 pass
- `cargo test -p backtest`: 113+ pass, 5 ignored (realdata), 0 failures
- `cargo test -p agent`: 76+ pass, 0 failures
- `cargo clippy -p strategy -p backtest -p agent -- -D warnings`: clean
- `scripts/verify_anchors.sh`: ANCHORS PASS (119/119)

## UI

_ui-designer, 2026-06-21 (claude-opus-4-8). UI tasks U-T1 / U-T2 / U-T3
implemented in `crates/ui` only — `cargo tree -p ui` byte-unchanged (469
packages; data crosses as the existing `backtest::BakeoffReport` /
`agent::config::ForwardPlan` mirrors)._

### Seam reconciliation (developer ‖ ui-designer parallel) — RESOLVED

The brief asked me to report the exact variant set I assumed so the developer
could reconcile. **One divergence found and resolved by matching the developer's
SHIPPED shape:**

- `agent::config::PlanRuleKind::Ensemble` ships as
  **`{ method: PlanVoteMethod, member_count: u32 }`** — NOT the ADR-0063 § D3
  prose's `members: Vec<PlanRuleShape>`. The developer chose a `Copy`-preserving
  scalar `member_count` (their doc, `crates/agent/src/config.rs:142`: "members
  are NOT recursively embedded here (that would break `Copy`) … full member
  detail is available from `strategy::EnsembleStrategy::describe_plan` … (v0.2
  extension point)").
- `agent::config::PlanVoteMethod` ships as **`Majority { k: u32, n: u32 }` |
  `Unanimous { n: u32 }`** (`u32`, NOT the ADR's `usize` — the developer narrows
  to `u32` at the `agent` boundary to stay `Copy + Eq` without `Decimal`).

The `ui` mirror matches the shipped shape **field-for-field**:
- `ui::forward_plan::PlanRuleView::Ensemble { method: PlanVoteMethodView,
  member_count: u32 }` (the enum stays `Copy`).
- `ui::forward_plan::PlanVoteMethodView::Majority { k: u32, n: u32 } |
  Unanimous { n: u32 }`.
- The `live`-gated adapter `forward_plan/adapter.rs::vote_method_view` maps the
  two `agent` variants one-for-one (the single reconciliation edit site).

**Consequence for the plan copy:** because the agent boundary delivers only
`method` + `member_count` (not each member's own rule), the F6 ensemble plan
describes the vote at the **consensus level** (method + live tally + caveat), not
a per-member rule list. A per-member list ("MACD trend — buys when …") is a
**v0.2 enhancement** gated on the agent boundary carrying the members — recorded
in `strings.rs` next to `FORWARD_PLAN_RULE_ENSEMBLE_CAVEAT` and in the
`PlanRuleView::Ensemble` doc.

### New screens / panels / widgets

No new screen — both surfaces are EXTENSIONS of existing screens:

- **Leaderboard** (`screens/leaderboard.rs`) — ensemble rows now render a
  **friendly vote label** (`display_label`: `v0.8.vote.majority` → "Majority vote
  (2-of-3)", `v0.8.vote.unanimous` → "Unanimous vote (4-of-4)") + a `vote` tag
  (`is_ensemble_id`). The **Fragile flag is now a prominent BADGE**
  (`fragile_badge`: a `DOWN_50`-tinted pill + saturated `DOWN_500` "fragile"
  label + `PILL` radius — the `Negative` status-pill intent from
  `ui-design-principles.md`), upgraded from the prior plain `SMALL` warn text
  because F8 makes the Fragile state non-inert and ineligible-to-crown, so it
  must be unmistakable in pixels. Robust / marginal stay plain muted text.
- **Forward plan** (`screens/forward_plan.rs`) — a new `Ensemble` arm in
  `rules_block` renders the vote FAITHFULLY: a headline IF/THEN vote rule
  (`ensemble_vote_clause`, so the conditional ACCENT paints like the singles) +
  a live tally (`ensemble_tally_line` — "Current vote: 2 of 3 … → Long") + the
  honest "this is a vote, measured like everything else" caveat + the cadence
  line. NOT a fabricated single-indicator rule. The not-a-prediction / not-advice
  framing is unchanged.

### New strings in `ui::strings`

Leaderboard: `LEADERBOARD_ENSEMBLE_MAJORITY_LABEL`,
`LEADERBOARD_ENSEMBLE_UNANIMOUS_LABEL`, `LEADERBOARD_ENSEMBLE_VOTE_TAG`.
Forward plan: `FORWARD_PLAN_RULE_ENSEMBLE_MAJORITY_FMT`,
`FORWARD_PLAN_RULE_ENSEMBLE_UNANIMOUS_FMT`, `FORWARD_PLAN_RULE_ENSEMBLE_TALLY_FMT`,
`FORWARD_PLAN_RULE_ENSEMBLE_CAVEAT`. (All registered in `strings::all()`; the
`LEADERBOARD_FRAGILE_TAG` already existed and is reused by the new badge.)

### New theme tokens

**Zero.** The Fragile badge composes existing tokens (`color::DOWN_50`,
`color::DOWN_500`, `radius::PILL`, `space::XXS`/`XS`) — the status-pill pattern
already defined in the design system.

### Render proofs (the verification floor — CLAUDE.md cockpit rule)

Real PNGs saved + READ (a passing pixel proxy is not proof until the image is
read):

- `/tmp/forward_f8_leaderboard_render.png` — the populated 7-arm leaderboard
  (4 singles + 2 vote ensembles + buy-and-hold) with the ensemble rows legible
  AS votes + the Fragile badge visibly rendered on the flagged `v0.5.rsi` AND
  the `majority vote (2-of-3)` rows, the crown (★ best) on the robust `v0.sma`
  (NOT on the Fragile ensemble — the credibility lock). Test:
  `leaderboard_f8_ensembles_and_fragile_badge_paint` (+ anti-tautology
  `leaderboard_f8_strictly_exceeds_five_arm_field`) in
  `crates/ui/tests/leaderboard_populated_render.rs`.
- `/tmp/forward_f8_ensemble_plan_render.png` — the ensemble forward plan
  (method + live tally "Current vote: 2 of 3 → Long" + caveat + sizing +
  horizon), with `/tmp/forward_f8_ensemble_plan_buyhold_control.png` as the
  buy-and-hold negative control (same KIND of object, NO IF/THEN vote accent).
  Tests: `forward_plan_f8_ensemble_paints_vote_rule_and_tally` +
  `forward_plan_f8_buy_and_hold_is_the_negative_control` in
  `crates/ui/tests/forward_plan_populated_render.rs`.

Pixel-count band assertions discriminate ensemble-present / Fragile-present vs
absent (the Fragile-badge clay is scoped to the STRATEGY column, x <
`STRAT_COL_RIGHT`, so the always-negative Max-DD column never confounds it).

### Accessibility notes

- The Fragile badge carries the **word "fragile"** (not colour alone) — colour
  is never the only signal. Contrast: `DOWN_500` label on `DOWN_50` backdrop is
  the `Negative` status-pill pair already verified in `tests/contrast.rs`.
- Both surfaces render under `--theme dark` and `--theme light` (the
  `ModeColor::current(mode)` token path; the render harness exercises Dark, the
  design-system tokens carry Light).
- No new interactive element (the rows + plan are read-only); existing keyboard
  nav / focus order is unchanged.

### Gates

`cargo fmt -p ui --check` clean; `cargo clippy -p ui --features fixtures --tests
-- -D warnings` clean (forced re-lint via `touch lib.rs`); full `ui` suite 40
test-binaries / 0 failures; no visual-snapshot baseline regenerated (the
leaderboard is covered by the dedicated `leaderboard_populated_render.rs` pixel
harness, not the `panel_snapshots` / `visual_snapshots` baselines — 108 + 51
pass unchanged).

## Verification
_tester links to reports here. Verification floor (provisional): (1)
`bakeoff_e2e`-style test proving the 2 ensembles enter the field, are ranked by
the unchanged comparator, and that `BenchmarkWins`/`AllFragile` stay reachable
with ensembles present; (2) the day-1 equity-divergence e2e (R4/Backtest
Scenarios); (3) a real-robustness test proving an overfit ensemble is flagged
Fragile and loses the crown to a robust single (requires `RobustnessMode::
Bootstrap`); (4) `verify_anchors.sh` 119/119; (5) Leaderboard + F6-plan
render-layer PNG showing an ensemble row + an ensemble plan (per the CLAUDE.md
iced pixel-layer rule)._

## Non-goals (consolidated — the honesty guardrails)

1. **No weight/threshold/membership search.** The candidate set and every vote
   rule are fixed code constants. No runtime tuning, no "best mix" optimizer, no
   user-built mixes.
2. **Ensembles are never assumed-better.** Same comparator, same Fragile gate,
   same buy-and-hold benchmark; `BenchmarkWins`/`AllFragile` stay reachable.
3. **No resurrecting retired ML as asserted alpha.** The retired TCN/PatchTST/
   GARCH/LLM-forecaster chains stay opt-in/retired; they are not vote members and
   not ranking inputs.
4. **LLM stays narration-only in v1.** No LLM signal in the ranking path; the
   LLM-arbiter is a separate, later, separately-gated feature.
5. **No new anchored scenario; no anchored-report edit.** 119/119 stays.
6. **No multi-coin, no live trading.** Single coin, paper-only — unchanged
   product constraints.

## Open questions

See the analyst handoff for the prioritized split. In brief:

- **Operator product-forks (must be answered before the architect designs):**
  OQ-OP-1 combination method (signal-vote recommended vs capital-split fallback);
  OQ-OP-2 LLM/ML participation in v1 (narration-only recommended vs wire-as-signal);
  OQ-OP-3 candidate-set size (two pre-registered votes recommended).
- **Architect-bound:** OQ-ARCH-1 the `EnsembleStrategy: Strategy` adapter seam
  (forward-run + dispatch); OQ-ARCH-2 the `RobustnessMode::Bootstrap` compute mode
  + whether the gate needs multiple-comparisons hardening at a 7-arm field;
  OQ-ARCH-3 the `PlanRuleShape::Ensemble` enum extension + `ui` exhaustive match.

## Changelog

- 2026-06-21 (architect, F8 design — ADR-0063): resolved OQ-ARCH-1..3 + the two
  hazards. OQ-ARCH-1 = `strategy::EnsembleStrategy` IMPLEMENTS the frozen
  `Strategy` trait (the `RegimeDispatcher` precedent generalised to N homogeneous
  members + a pure `arbitrate(method,&stances)->Stance` arbiter) ⇒ the ONE
  primitive reachable identically from `run_scenario` + `build_registry_for`
  (F5b) + `StrategyRegistry`; members REUSED via a shared `build_member(id)` over
  the SAME `config/strategies/*.toml`; the WARMUP rule = an un-warmed member
  ABSTAINS (counted in neither `long_count` nor the denominator; ensemble FLAT
  until its quorum is warm — treating `None` as FLAT rejected as manufacturing
  false majorities). OQ-ARCH-2 = `RobustnessMode::Bootstrap { paths:1000, seed }`
  compute-and-feed: equity→returns, block length via the EXISTING Politis–White
  selector, ADR-0051 D1 sub-seed determinism (`master + j*0x9E3779B9`, master
  derived from the bake-off seed), reduce via `DistributionSummary::
  from_path_metrics`, classify via the FROZEN `classify_verdict`; multiple-
  comparisons re-evaluated and DEFERRED (not-yet at a fixed 7-arm pre-registered
  field — the gate is an absolute per-candidate band rule not a best-of-K p-value;
  ADR-trigger recorded for a growing set / selection-over-configs). OQ-ARCH-3 =
  `PlanRuleShape::Ensemble { method: PlanVoteMethod, members: Vec<PlanRuleShape> }`
  closed-enum extension + `ui` exhaustive `Ensemble` arm naming method+members
  (no `String` crosses the seam). ANCHOR SAFETY 119/119 by construction: `Skip`
  stays the default, the two arms + `Bootstrap` are opt-in ONLY on the no-report
  advisor path, existing single-id arms + `default_field()` byte-untouched. TWO
  day-1 e2e gates specified (the vote combines; the gate bites / anti-`Skip`).
  Verified baseline `verify_anchors.sh` 119/119 (touched zero report files).
  Wrote ADR-0063 + registered it in the ADR README (atomic). Trace
  `REQ-ADVISOR-ENSEMBLE-001` `arch` filled; state → `architected`. HANDOFF →
  developer ‖ ui-designer. Tasks split in
  [tasks.md](tasks.md).
- 2026-06-21 (analyst, F8 scoping — NEW feature folder): authored the honest F8
  definition (ensemble = a bounded, pre-registered set of deterministic
  signal-vote candidates that earn their crown through the SAME comparator + the
  SAME Fragile gate + the SAME buy-and-hold benchmark as the singles), proposed
  the **signal-vote** combination method (reuses `StrategyRegistry::on_bar`
  fan-out; one curve → one robustness flag; honestly plan-describable) with
  **capital-split named as the v0.3 fallback**, fixed the **bounded candidate set
  to two pre-registered votes** (`v0.8.vote.majority` ≥2-of-3, `v0.8.vote.
  unanimous` 4-of-4), and ratified **LLM/ML as narration-only in v1** after
  verifying against code that (a) `crates/llm` is imported by neither `strategy`
  nor `backtest` and produces no `Signal`/`Direction` — it is genuinely narration
  plumbing, the bake-off "LLM-as-analyst arm" is aspirational not built; and (b)
  the only `ForecastProvider` impls are the retired TCN/PatchTST/GARCH/LLM-
  forecaster chains (concluded not-beating-passive) — so no ML may be resurrected
  as asserted alpha. Flagged for the architect: the inert robustness gate
  (`RobustnessMode::Skip` only → nothing is ever Fragile today) MUST gain a real
  `Bootstrap` compute mode before ensembles ship, the `EnsembleStrategy: Strategy`
  adapter as the durable forward-run/dispatch seam, and the `PlanRuleShape::
  Ensemble` enum extension for F6. NON-goals fixed (no weight search, no
  assumed-better, no retired-ML-as-alpha, LLM narration-only, 119/119 anchors,
  single-coin paper-only). No engine code; no anchored content touched. Trace row
  `REQ-ADVISOR-ENSEMBLE-001` created (state `proposed`). Sibling of
  REQ-ADVISOR-BAKEOFF-001 (F1–F3), REQ-ADVISOR-FORWARD-PAPER-001 (F4–F5),
  REQ-ADVISOR-FORWARD-PLAN-001 (F6), REQ-ADVISOR-DYNAMIC-DATA-001.
- 2026-06-21 (ui-designer, F8 UI — U-T1/U-T2/U-T3): added the `## UI` section.
  Leaderboard ensemble rows (friendly "Majority/Unanimous vote" labels + `vote`
  tag) + the Fragile flag upgraded to a prominent `DOWN_50`/`DOWN_500` PILL
  badge (the first non-inert Fragile pixel — shown-but-not-crowned). F6 ensemble
  forward-plan `Ensemble` arm renders the vote faithfully (method + live tally +
  caveat), NOT a fabricated single rule. **Seam reconciliation:** matched the
  developer's SHIPPED `agent::config::PlanRuleKind::Ensemble { method,
  member_count: u32 }` + `PlanVoteMethod { Majority{k,n}|Unanimous{n} }` (`u32`)
  — NOT the ADR's `members: Vec<…>`/`usize`; `ui` mirror is field-for-field
  (`PlanRuleView::Ensemble { method, member_count }`, stays `Copy`); per-member
  rule list deferred to v0.2 (agent boundary carries only `member_count`). Render
  proofs READ: `/tmp/forward_f8_leaderboard_render.png` (7-arm + Fragile badge,
  crown on robust `v0.sma`) + `/tmp/forward_f8_ensemble_plan_render.png` (+ a
  buy-and-hold negative control). Zero new theme tokens; `cargo tree -p ui`
  byte-unchanged (469 pkgs). Gates: fmt clean, clippy `-D warnings` clean (forced
  re-lint), full `ui` suite 40 binaries / 0 failures, no visual baseline
  regenerated. `crates/ui` only — touched no engine crate, no `spec/*/reports/`,
  no `tasks.md`.
- 2026-06-22 (tester): independent verification complete. Commit
  `c16a37ca507e8c8d5a37bf7598cdec819b4a3c25`. All gates PASS: 12 ensemble vote
  divergence e2e tests + 15 robustness bootstrap tests (both day-1 mandatory
  gates); 11 leaderboard render tests (ensemble rows + Fragile badge); clippy -D
  warnings clean workspace-wide; fmt clean; anchors 119/119. Status bumped to
  `shipped`. Report:
  `spec/advisor-ensemble/reports/test-advisor-ensemble-2026-06-22.md`.
