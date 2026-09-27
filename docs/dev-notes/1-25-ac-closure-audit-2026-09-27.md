# Story 1-25 — AC closure audit, 2026-09-27

**Question put to this audit:** can `1-25-harness-fill-correctness-relock` honestly flip from
`in-progress` to `done`, given that AC4 + AC5 were split out into story 1-26 (now `done`)?

**Answer: NOT CLOSABLE.** One AC is *self-documented as unmet in the source at HEAD*, and five of
the seven AC3 riders are neither fixed nor ratified anywhere in the story record. Details below;
the blocking list is § 4.

---

## 0. Provenance — what backs each verdict

| class | what it covers |
|---|---|
| **READ (code)** | `crates/backtest/src/paper.rs`, `engine.rs`, `stats/mod.rs`, `mc_harness.rs`, `resample.rs`, `scenarios/montecarlo.rs`, `scenarios/threshold_sweep.rs`, `bin/monte_carlo.rs`, `bin/threshold_sweep.rs`, `bin/param_robustness_sweep.rs`, `crates/risk/src/portfolio.rs`, `crates/forecast/src/bin/sharpe_comparison.rs`. `scenarios/threshold_sweep.rs::run_cell` was read **end to end** (all 360 lines), not sampled. |
| **READ (artifacts)** | the 1-25 and 1-26 story files; `docs/dev-notes/bug-log.md` entries `#67`–`#69`, `#71`–`#76`, `#89`, `#94`, `#95`, `#110`, `#111`, `#126`, `#127`; `docs/dev-notes/1-25-architect-seam-and-relock-plan-2026-08-16.md`; `evidence/v2/harness-relock/ERRATA.md` (all 9 sections); ADR-0089; `evidence/anchors.toml`. |
| **RAN (test)** | `cargo test -p backtest --lib paper::tests` → **12 passed / 0 failed / 0 ignored**. `cargo test -p backtest --test portfolio_controls_bind` → **3 passed / 0 failed / 0 ignored**. |
| **RAN (tooling)** | `scripts/callers.sh run_cell`, `scripts/callers.sh run_path`; `git show --stat` on `11acd12`, `936134c`, `7884f52`, `723ca742`, `ae62de8`; `git log -- crates/backtest/src/bakeoff/bootstrap.rs`; `grep -rl NaN evidence/*/reports/`. |
| **INFERENCE** | flagged inline as *(inference)*. Nothing was mutated to prove a test binding — this audit is read-only on `crates/`, so binding claims rest on the test's own structure (e.g. a `.unwrap_err()` cannot pass without the guard), not on a neutering run. Where the repo's own commit message records a neutering run, that is cited as such. |
| **NOT RUN** | no anchored surface was regenerated; no binary was executed. Nothing was written into `evidence/`. |

One correction to the audit brief itself: **`run_cell` is not in `bin/threshold_sweep.rs`.** It is
`crates/backtest/src/scenarios/threshold_sweep.rs:56`. The bin's `fn @643` is the *caller*;
`scripts/callers.sh run_cell` lists the three call sites as `bin/threshold_sweep.rs:783`, `:845`,
`:937`, all inside that function. The `solvenc|pre_flight|preflight` grep returned nothing because
it was pointed at the caller, not the callee — but the answer does not change: see AC2.

---

## 1. Per-criterion verdicts

| # | criterion | verdict | evidence |
|---|---|---|---|
| **AC1** | fill-symbol correctness, typed reject, bound by a test | **MET** (with a scope note) | see § 2.1 |
| **AC2** | BOTH lanes fixed; `run_cell` gains the Bug-B solvency guard | **NOT MET** | see § 2.2 |
| **AC3.1** | √8760 annualization, or ratified √8575 + doc corrected | **NOT MET** (half-ratified in a test comment, doc still false) | § 2.3 |
| **AC3.2** | hashed-body verdict vocabulary → frozen 5-signal rule | **NOT MET** | § 2.4 |
| **AC3.3** | sentinel-zero pooling policy | **NOT MET** — no policy exists in any artifact | § 2.5 |
| **AC3.4** | negative-final Calmar guard | **PARTIAL, unratified** — clamped on 3 lanes, unclamped on the `run_cell` lane; the assertion is release-inert | § 2.6 |
| **AC3.5** | slippage-aware solvency pre-flight | **NOT MET** — symptom guarded downstream, rider not implemented | § 2.7 |
| **AC3.6** | FILL_SEED domain separation | **NOT MET** — the cited constants are the defect, not the fix; moot-ness documented in dev-notes, not ratified | § 2.8 |
| **AC3.7** | real portfolio-exposure-cap enforcement (enforce or delete) | **MET on the inventory lane**; OPEN on 8 others, gap disclosed in writing | § 2.9 |
| **AC6** | advisor-gate independence re-proven by assertion | **NOT MET as written** — credited to a test that does not measure the claim; the claim is nonetheless true on other evidence | § 2.10 |
| — | stale checkboxes at story lines 97-103 and 104-124 | **BOTH STALE** — supersession confirmed; one orphaned sub-obligation found | § 3 |

---

## 2. The evidence, criterion by criterion

### 2.1 AC1 — MET

The guard exists and is typed:

- `crates/backtest/src/paper.rs:148-155` — the loop `for order in &orders` returning
  `MatchError::SymbolMismatch`, placed **before** any pricing work.
- `crates/backtest/src/engine.rs:68-72` — the error variant, with the ratified-seam rationale in its
  doc (reject, never silent deferral).

Three tests bind it, none `#[ignore]`d, none cfg-gated beyond the ordinary `#[cfg(test)]` at
`paper.rs:257`:

| test | line | binds what |
|---|---|---|
| `paper_step_rejects_order_for_a_different_symbol` | `paper.rs:561` | the guard fires |
| `paper_step_symbol_guard_is_noop_on_single_symbol_batches` | `paper.rs:589` | AC1's no-op proof, pinned to **literal** arithmetic (close 0.30 + 2 bps), so it cannot degenerate into a two-run tautology |
| `paper_step_symbol_mismatch_is_all_or_nothing` | `paper.rs:610` | a mixed batch fills nothing |

RAN: 12 passed / 0 failed / **0 ignored**. The first test is structurally binding — it calls
`.unwrap_err()`, so deleting the guard makes it panic rather than pass *(inference from reading, not
from a neutering run)*.

**Scope note, and it matters for AC2.** AC1's own wording is *"prices each order at ITS symbol's bar
… (or defers the order to that symbol's bar)"*. That is satisfied on `run_path` only, via
`last_bar_by_symbol` (`scenarios/montecarlo.rs:351`, `:356`, `:537`) plus a `match` on the step
result that surfaces a rejection (`:582`+). Every **other** multi-symbol production lane still calls
`engine.step` inside a `let Ok(fills) = … && …` chain with no else arm, no `warn!` and no counter —
`scenarios/threshold_sweep.rs:228` and `:269`, `scenarios/pairs.rs:248` and `:314`,
`scenarios/garch_vol_target_overlay.rs:305` and `:357`. There, a cross-symbol order is now **silently
dropped**: neither priced at its own bar nor deferred. That is bug-log `#111`'s finding stated from
the code side, and `#111` measured its consequence (`pairs-{2023,2024-h1}-zscore-mr` both DRIFTED).

### 2.2 AC2 — NOT MET (the single genuinely-unmet hard criterion)

AC2 has three clauses. One is discharged, two are not.

**(a) `run_cell` was not fixed in AC1's sense.** Read end to end: no `last_bar_by_symbol`, no
per-symbol routing, no rejection handling. It got the engine guard's *rejection* and the swallow
chain turns that into a silent no-op.

**(b) The Bug-B solvency guard was never added — and the source says so at HEAD.**
`crates/backtest/src/scenarios/montecarlo.rs:5-8`, verbatim:

> `//! since DIVERGED: from v0.1.1 run_path carries the Bug-B long-only`
> `//! solvency guard (pre-flight cash check + fill-loop guard), which`
> `//! run_cell does NOT — run_cell retains the pre-Bug-B unguarded Buy`
> `//! sizing inside the frozen-anchored threshold-sweep lane.`

Confirmed independently by reading `run_cell`. Its only cash-adjacent check is
`if equity <= Decimal::ZERO { continue; }` at `scenarios/threshold_sweep.rs:196` — and `equity =
cash + position_value` (`:195`), so it does not bound cash at all. Buy sizing is
`let notional = equity * dec!(0.10)` (`:207-208`) with no comparison against `cash` anywhere before
`engine.step` at `:228`. With up to ten concurrent 10 %-of-equity legs plus fees, cash goes negative;
that *is* Bug-B. So AC2's parenthetical — *"run_cell additionally gains the Bug-B solvency guard it
never received"* — is the named deliverable of this AC and it is absent.

This lane is not anchor-free: it produces `threshold-sweep-bs1-realdata-recalibrated` and
`threshold-sweep-bs2-realdata-recalibrated` (`evidence/anchors.toml:262`, `:267`). The story's own
Task note (line 87-88) says correctly that `run_cell` produces **none of the 34 inventory anchors** —
true, and not the same as producing none.

**(c) The FROZEN-gate clause IS discharged.** `git show --stat` on all five story commits
(`11acd12`, `936134c`, `7884f52`, `723ca742`, `ae62de8`) touches none of `bakeoff/robustness.rs`,
`bakeoff/rank.rs`, `bakeoff/bootstrap.rs`. `evidence/v2/harness-relock/ERRATA.md` § 8 records the
same, and `fdr_annex_identity` (2 passed, per that table) is the identity test.

### 2.3 AC3.1 — √8575 vs √8760: NOT MET

At HEAD, `crates/backtest/src/stats/mod.rs`:

- `:42` and `:72` — `const SQRT_HPY: f64 = 92.601_295_098_46;` whose square is **8574.9999…**
- `:35` — the doc directly above it: `` /// Formula: `mean_log_return` / `std_log_return` * sqrt(24*365). `` — i.e. **8760**. Still false.

The forecast-crate twin is identical in both halves: `crates/forecast/src/bin/sharpe_comparison.rs:113`
holds the constant, `:109` documents it as `√(24 · 365) ≈ 92.601295`, and the renderer prints
`sqrt(24*365) = {:.6}` into report bodies at `:1007`, `:1345`, `:1681`. The test that the design named
the re-sync trigger, `crates/forecast/tests/sharpe_comparison_determinism.rs:41`, **re-declares the
constant in its own file** and compares against that — so it can never trip on the doc/const
mismatch. (2-18 finding F4, confirmed live.)

**Is the rider superseded by `resample.rs`?** No. `crates/backtest/src/resample.rs:151-158`
(`bars_per_year_1h` → 8784 leap / 8760 otherwise) is a different code path: its own doc names its two
consumers as *"the sweep's expected-bar-count arithmetic and `Horizon::periods_per_year`"*, and
`periods_per_year` feeds the `compute_*_periodic` 4h/daily siblings. The **1h** lane — where the
anchors and the operator-facing Baseline comparator live — still multiplies by the hardcoded
`SQRT_HPY`. The rider stands.

**How far the alternative disposition got.** AC3.1 permits *"a formal ratification of √8575 with the
doc corrected to match"*. The value **is** ratified informally, in a test comment:
`stats/mod.rs:930-945` states *"the constant is √8574.9998…, ~2.1 % away"* from √8760 and pins it as
anchor-load-bearing, RED-on-revert; `f_hr_1_compute_sharpe_hourly_value_unchanged` (`:946`) enforces
it. So the half that is missing is exactly the half AC3.1 names: **the doc correction**, plus a
ratification in the story record rather than in a comment. Note also that the strings live in
*hashed bodies*, which makes this bug-log `#110`'s shape — a corrected engine through an unchanged
renderer says the same words, so this cannot be cleared by a re-lock alone.

*Unverified:* the story's Dev Notes (line 151) add a cockpit re-sync obligation at
`crates/ui/src/baseline/loader.rs`. That file now contains **no** annualization constant (grep for
`92.60|8760|8575|sqrt|SQRT|annual` → nothing relevant). The obligation's target has moved and I could
not locate the current cockpit const. Listed in § 5.

### 2.4 AC3.2 — verdict vocabulary: NOT MET

`crates/backtest/src/bin/monte_carlo.rs:330-341` still renders **into the hashed body**:

- `ROBUST-ABOVE-1` when `sharpe_p50 > 1.0`
- `MARGINAL` when `sharpe_p50 > 0.0`
- `WEAK` otherwise

That is a **single-signal p50 rule**. The frozen rule is
`crates/backtest/src/bakeoff/robustness.rs:120 classify_verdict` — 5-signal weakest-link, vocabulary
`FRAGILE` / `MARGINAL` / `ROBUST` (`robustness.rs:64`). `WEAK` and `ROBUST-ABOVE-1` do not exist in
the frozen vocabulary, and `FRAGILE` never appears in the monte_carlo body at all.

Worth recording: the **C3** lane is already aligned — `sweep_harness.rs:2307` prints *"Verdict:
FRAGILE/MARGINAL/ROBUST via 5-signal weakest-link (frozen decision-rule § 0 bands)"*. It is the
**C2** lane that is not. Neither fixed nor ratified.

### 2.5 AC3.3 — sentinel-zero pooling: NOT MET, and no policy exists anywhere

Four distinct "undefined" conditions all return the same `0.0`:
`compute_sharpe_hourly` at `stats/mod.rs:45` (`n < 2`) and `:58-60` (`std < 1e-15`);
`compute_sortino_hourly` at `:75`; `compute_calmar` at `:104`, `:109`, `:113`, `:118`.

`reduce_samples` (`stats/mod.rs:530-591`) then pools those values straight into `p5`/`p50`/`p95` as
if they were observations. It is demonstrably *NaN- and ±Inf-aware* — `reducer_nan_fails` (`:802`),
`reducer_rejects_positive_infinity` (`:818`), `reducer_rejects_negative_infinity` (`:837`) — and has
no notion of a sentinel at all.

Searched for the policy the AC asks for: `grep -rn "sentinel-zero\|sentinel zero\|pooling policy"`
over `docs/` and `_bmad-output/` returns **only the four places the rider is listed** (bug-log
`#67`'s rider line, the architect plan, 1-25 AC3, 1-26's Dev Notes) — i.e. four restatements of the
obligation and zero dispositions. Neither fixed nor ratified.

### 2.6 AC3.4 — negative-final Calmar guard: PARTIAL, unratified

`compute_calmar` (`stats/mod.rs:100-122`) guards `initial <= 0.0` (`:108`) but **not a negative
final**: `(final_eq / initial).powf(1.0 / years)` at `:115` with a negative base and a fractional
exponent is NaN. The real protection is a caller-side clamp of `e <= 0 → dec!(0.000001)` at three
sites — `mc_harness.rs:294-303`, `bin/param_robustness_sweep.rs:911-917` and `:1974-1980` — the exact
three enumerated by bug-log `#127` (disclosed 2026-09-27, owned by story **1-28**, which also rules
the clamp *"correct and staying"*).

Two things keep this from being a discharged criterion:

1. **The assertion beside the clamp is release-inert.** `debug_assert!(calmar.is_finite(), …)` at
   `mc_harness.rs:318` and `param_robustness_sweep.rs:948` is compiled out of the `--release` builds
   that produce every anchored surface. Under the house rule, that leg is a gate that cannot fail in
   the runs that matter.
2. **The `run_cell` lane has no clamp at all.** `grep -n "clamp\|0.000001\|<= Decimal::ZERO"` over
   `crates/backtest/src/bin/threshold_sweep.rs` returns **nothing**, and it calls
   `compute_calmar(&result.equity_curve)` on the raw curve at `:790` and `:945`, printing the result
   into the hashed body at `:571-572`.

Those two AC-level gaps compound: **AC2(b)'s missing solvency guard is the mechanism that produces
the negative final equity AC3.4's missing guard would have to survive.** No solvency guard → cash
goes negative → final equity negative → `powf` → `NaN` printed as a Calmar in a hashed body.

Empirically **not yet manifest**: `grep -rl "NaN" evidence/*/reports/` returns nothing, so the frozen
corpus contains no NaN today. Latent, not live — but unratified either way.

### 2.7 AC3.5 — slippage-aware solvency pre-flight: NOT MET

`scenarios/montecarlo.rs:558-565`:

```
let required_cash = match side {
    Side::Buy => Some(notional + fee_estimate),
    …
```

with `fee_estimate = notional * Decimal::new(i64::from(taker_fee_bps), 4)` at `:542`. **No slippage
term.** `slippage_bps` is read at `:197` and handed to `MatchConfig` at `:218`, and never enters the
pre-flight. The engine then fills a Buy at `base_price * (1 + slippage/10_000)`
(`paper.rs:170-172`), so the pre-flight understates the cash actually required by
`notional * slippage_bps / 10_000`.

The *symptom* is caught downstream: the fill-loop guard at `montecarlo.rs:587-604` compares
`total_cost > cash` on the real fill and skips with a `warn!`, so cash cannot go negative on this
lane. The *rider* — make the pre-flight slippage-aware, so the decision is taken before the order
exists rather than by abandoning a fill — is not implemented and not ratified.

### 2.8 AC3.6 — FILL_SEED domain separation: NOT MET

The two constants named in the brief are **the defect, not the fix**:
`crates/backtest/src/bin/monte_carlo.rs:379` and `crates/backtest/tests/montecarlo_e2e.rs:51`, both
`const FILL_SEED: u64 = 0xC0FFEE`. Story 1-14's record names the rider precisely as *"FILL_SEED ==
default master seed domain collision (inert today)"*. And the collision is real and wide: the same
literal is the project-wide fixture seed at ~25 sites — `core/src/forecast.rs:120` (`sampling_seed`),
`PaperEngine::new(config, 0x00C0_FFEE)` throughout `paper.rs`'s tests,
`ui/tests/lab_markers_anchor.rs:29`, `llm/tests/prompt_cache_test.rs:65`, and so on. No domain
separation of any kind exists (no tag, no hash, no derivation).

There **is** a written argument that the rider is moot — the architect plan
(`docs/dev-notes/1-25-architect-seam-and-relock-plan-2026-08-16.md:94-96`) and 1-26's Dev Notes
(`:193-194`), both citing bug-log `#89`: *"the PaperEngine seed is currently inert — domain-separating
a value nothing reads buys nothing until it is wired"*. That argument is sound. It is also **not a
ratification in the story record**, which is what AC3 requires, and `#89`'s own status line says the
wire-or-delete decision *"remains the operator's"* — so the premise is an open question, not a
settled one. This is the cheapest rider to close honestly: one ratification line in the story.

### 2.9 AC3.7 — portfolio-exposure-cap: MET on the inventory lane, OPEN on eight others

**Fixed, and verified rather than taken on trust:**

- bug-log `#69` status line: *"**FIXED 2026-08-23** (`723ca742`), verified 2026-09-25"*, and it
  records that `scripts/callers.sh size_portfolio_target` now reports the production caller
  `scenarios/montecarlo.rs` inside `run_path`, where its own earlier census found zero.
- bug-log `#68` status line: *"**RESOLVED 2026-09-25 — implemented, not dropped**"*, operator-ruled
  (*"Zählt als erfüllt"*); the drift axis stayed and became live as a side effect of the same wiring.
- The gross + signed-leg sizer exists: `crates/risk/src/portfolio.rs:83` (`GROSS, not long-only`),
  `:103`, `:129`, `:183-206`, `:245`. ADR-0089 **D7** is at
  `_bmad-output/planning-artifacts/architecture/decisions/0089-harness-portfolio-sizer-wiring.md:92`.
- Three binding gates, `crates/backtest/tests/portfolio_controls_bind.rs`. RAN: **3 passed / 0 failed
  / 0 ignored**. The commit message for `723ca742` records each one RED-proven by neutering
  (cap check disabled; `relative_drift > threshold` forced true; `#94` reverted → 50 fills vs 10).

**Open:** bug-log `#95` status line is *"**OPEN** — scope finding, needs an operator ruling"*.
`portfolio_exposure_cap` is declared at **9** sites and read at **1** (`portfolio.rs:243`, inside
`size_portfolio_target`); `Order::new` never consults it. Eight lanes still declare a cap that cannot
bind — including `run_cell` itself at `scenarios/threshold_sweep.rs:156` (`Some(dec!(0.50))`) and
`pairs.rs` at `0.75`. The 1-17 escalation recorded in this story (line 164) widened AC3 to
*"enforce-or-delete + a BINDING test for **every** declared risk limit"*, which those eight do not
satisfy.

The gap **is** disclosed in writing, which is why this reads as MET-with-a-residual rather than
unmet: `evidence/v2/harness-relock/ERRATA.md` § 7 states *"eight lanes still declare a
portfolio_exposure_cap they cannot enforce… **This errata makes no claim that the exposure cap binds
engine-wide.** It binds on `run_path`, which is the lane behind all 34 of these surfaces."*

### 2.10 AC6 — advisor-gate independence: NOT MET as written

1-26's errata § 8 credits this leg:

> `advisor-gate independence | robustness_bootstrap_bites **17 passed** — bakeoff/bootstrap.rs inputs and outputs unchanged`

Reading `crates/backtest/tests/robustness_bootstrap_bites.rs`: it is ADR-0063's **pre-existing**
D-T5.2 behavioural gate — declining equity ⇒ FRAGILE, growing ⇒ not, same-seed determinism,
short/empty ⇒ Skipped, `derive_master_seed` deterministic and per-candidate unique. It pins **no
numeric baseline** and asserts **nothing about its inputs**. It would pass identically whether or not
the candidate equity curves that feed the bootstrap had moved. So the sentence *"inputs and outputs
unchanged"* is over-read from a test that does not measure it — a gate whose success path is
unobservable for the claim it is cited for.

The claim itself is nonetheless **true** on evidence I verified independently:

- **Outputs:** `git log -- crates/backtest/src/bakeoff/bootstrap.rs` → last touched by `3c297b3e`
  (advisor-param-tuning). **None** of the five 1-25 commits touches it. Byte-level unchanged.
- **Inputs:** `scripts/callers.sh run_path` → the only production callers are `mc_harness.rs:281`
  and `bin/param_robustness_sweep.rs:894`; neither is on the advisor path. `crates/backtest/src/bakeoff/*.rs`
  never calls `engine.step`. Bakeoff arms dispatch per-coin (single-symbol), so the `#67` guard cannot
  fire there *(inference from the dispatch design, corroborated below)*.
- **Measured corroboration:** bug-log `#111`'s complete measurement lists
  `btc-2023-1m-sma-cross` and `btc-2023-1m-sma-baseline-refresh` as **gate-confirmed to reproduce**
  at HEAD — the single-symbol advisor-family lane demonstrably did not move.

So AC6 needs either the assertion it names, or a re-wording onto the evidence that actually carries
it. What it must stop doing is crediting `robustness_bootstrap_bites`.

---

## 3. Stale checkboxes — supersession CONFIRMED for both

All three entries read in full. The checked entry at story **lines 65-88**
(*"#68 + #69 — WIRED AND BINDING 2026-08-23"*) executes both unchecked entries below it.

**Box at lines 97-103** — *"#68 + #69 — UNITS RULED 2026-08-22: `exposure_cap` MEANS GROSS"*.
**STALE.** Its two substantive claims are both discharged:
1. The ruling is codified — ADR-0089 **D7** (`0089-…:92`).
2. Its stated blocker — *"`size_portfolio_target` **cannot implement the ruling as written**… it must
   be extended to signed weights with a gross cap, or replaced"* — is gone. The checked entry says
   *"The sizer was first extended to signed weights with a GROSS cap (D7), which is what made the
   ruling implementable"*, and `crates/risk/src/portfolio.rs:83-247` plus the green
   `gross_cap_refuses_the_whole_rebalance_and_the_count_is_surfaced` confirm it.

**One residual, and it is not 1-25's.** The box's closing sentence — *"1-26's errata owes the
per-scenario non-compliance record"* — is **undelivered**. I searched all nine sections of
`evidence/v2/harness-relock/ERRATA.md`: there is no per-scenario gross-non-compliance table (the
6 legs × 0.10 = 0.60-gross-vs-hashed-0.50 record). 1-26 is `done`. That obligation is **orphaned** —
assigned to a story that closed without it. It does not block 1-25, but it needs re-homing.

**Box at lines 104-124** — *"(prior) #68 + #69 are ONE defect — RE-RULED 2026-08-19: WIRE
`size_portfolio_target` FULLY"*. **STALE.** Self-labelled `*(prior)*`; its instruction (wire the
function fully, accept both controls, a binding test for each) is precisely what the checked entry
did. Bug-log `#68` closes it independently: *"The premise of this entry is void, verified
2026-09-25… #68 and #69 were ONE defect."*

**Genuinely open boxes, for completeness:**

| story line | box | reading |
|---|---|---|
| 39 | `[~]` Dev: fixes per AC1-AC3 | **honest.** Its own sub-list is complete; AC2(b) and riders AC3.1-AC3.6 are not. Keep `[~]`. |
| 125 | `[ ]` Re-run + re-lock + errata (AC4) | **not stale, but mis-marked.** Operator-split to 1-26 (`done`). Should read split-out, not open. |
| 138 | `[ ]` Review: old rows intact… (AC5) | same — 1-26 delivered AC5 (errata § 5, with the signal-trip table). |

So: **two boxes are stale bookkeeping (97-103, 104-124), two are mis-marked split-outs (125, 138),
one is honestly partial (39).** No unchecked box represents work that is both open and owned by 1-25
— *except* what AC2 and AC3 carry in their own right.

---

## 4. Verdict

### NOT CLOSABLE

The shortest list of work standing between HEAD and a closable story:

**Needs an operator ruling (1 item — the only one that is a real code/compute question):**

1. **AC2's `run_cell` clause.** The named deliverable is absent and the source says so
   (`montecarlo.rs:5-8`). Either
   **(a)** give `run_cell` per-symbol fill routing + the Bug-B solvency guard and re-lock the two
   anchors it produces (`anchors.toml:262`, `:267`); or
   **(b)** rule AC2 narrowed to `run_path`, recording in the story *why* the `run_cell` lane stays
   pre-Bug-B — and disclosing that the engine guard converted its cross-symbol orders from
   wrong-priced fills into **silent drops**, which is a behaviour change to an anchored lane that no
   artifact currently states.

**Needs writing only, no compute (5 items — AC3 permits "ratified-as-is in the story record"):**

2. **AC3.3 sentinel-zero pooling** — write the policy. Even *"pool as-is, here is why"* discharges
   it; today no artifact contains a disposition, only four restatements of the obligation.
3. **AC3.4 Calmar** — ratify the clamp as the guard, record that the companion `debug_assert!` is
   release-inert, and either close or ratify the unclamped `bin/threshold_sweep.rs:790`/`:945` path.
4. **AC3.5 slippage-aware pre-flight** — implement the slippage term at `montecarlo.rs:558-565`, or
   ratify the downstream fill-loop guard as sufficient, in writing.
5. **AC3.6 FILL_SEED** — one ratification line pointing at bug-log `#89`. The argument is already
   written in two dev-notes; it just is not in the story, and `#89`'s own decision is still open.
6. **AC6** — either add the assertion AC6 names, or amend AC6 to cite the evidence that carries it
   (`bootstrap.rs` byte-untouched across all five commits; bakeoff never calls `engine.step`;
   `#111`'s measured reproduction of the two `btc-2023-1m-sma-*` anchors) and stop crediting
   `robustness_bootstrap_bites`.

**Needs a renderer change + a re-emission ruling (2 items — bug-log `#110`'s protocol):**

7. **AC3.1 √8575** — correct the doc (`stats/mod.rs:35`; `sharpe_comparison.rs:109`) and ratify the
   value in the story record; the informal in-test ratification at `stats/mod.rs:930-945` covers the
   value but not the doc. The false `sqrt(24*365)` string is also rendered into report bodies
   (`sharpe_comparison.rs:1007`, `:1345`, `:1681`), so clearing it in the corpus is a prose rider:
   per `#110`, a re-lock alone cannot do it.
8. **AC3.2 verdict vocabulary** — align `bin/monte_carlo.rs:330-341` to the frozen 5-signal rule, or
   ratify the C2 lane's own vocabulary. Same prose-rider caveat; note C3 is already aligned
   (`sweep_harness.rs:2307`).

**Bookkeeping (4 edits, no decisions):**

9. Mark story boxes at lines 97-103 and 104-124 superseded (the pattern is already in the file at
   line 89); mark lines 125 and 138 split-to-1-26; re-home the orphaned per-scenario gross
   non-compliance record that 1-26 closed without.

**Cheapest honest path to `done`:** one operator ruling (item 1) + one ratification pass (items 2-6)
+ the bookkeeping (item 9), with items 7-8 either ratified in place or explicitly deferred to a
named re-emission story. Note that flipping 1-25 to `done` also requires its trace row
(`REQ-HARNESS-FILL-CORRECTNESS-001`, currently `state=scoped`) and a CHANGELOG line in the same pass
— ADR-0082's triad, enforced by `scripts/spec_lint.py`.

---

## 5. Unverified — stated as such

1. **The cockpit annualization re-sync target.** The story's Dev Notes (line 151) name a hardcoded
   const in `crates/ui/src/baseline/loader.rs`. No annualization constant is in that file at HEAD. I
   did not trace where the Baseline screen now gets Sharpe/Sortino from, so I cannot say whether that
   sub-obligation is discharged, moved, or dead.
2. **Whether the `run_cell` guard actually fires in production.** Mechanistically it must (the lane
   iterates merged multi-symbol bars and the strategy emits cross-sectional signals), and `#111`
   measured the analogous drop on `pairs-*` — but I did not run the `candle`+`realdata` sweep to
   observe it on this lane, so the *magnitude* on the two threshold-sweep anchors is unmeasured.
3. **Binding-by-neutering.** I did not mutate any source to RED-prove a test (read-only mandate).
   AC1's and AC3.7's binding claims rest on test structure and on the neutering runs recorded in
   `723ca742`'s commit message, not on runs of my own.
4. **The wider `#67` blast radius is still moving.** Bug-log `#111`/`#126` establish that the
   inventory was scoped by lane while the fix landed beneath the lanes; `#111`'s complete measurement
   (2026-09-26) lists **15** anchored scenarios gate-confirmed not to reproduce, and `#126` re-locked
   another one on 2026-09-27 (HEAD). That work is owned by 1-27 and successors, not by 1-25 — but no
   claim in this audit should be read as asserting the `#67` corpus is settled.
