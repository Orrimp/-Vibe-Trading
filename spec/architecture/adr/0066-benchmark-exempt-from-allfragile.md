---
adr: 0066
title: Benchmark exemption from the AllFragile outcome (rank_candidates amendment; classify_verdict UNCHANGED)
status: accepted
date: 2026-06-22
supersedes: none
superseded-by: none
---

# ADR-0066: Benchmark exemption from the `AllFragile` outcome determination (`rank_candidates` amendment; `classify_verdict` UNCHANGED)

## Context

F8 (ADR-0063) activated the robustness gate on the advisor bake-off path
(`RobustnessMode::Bootstrap`). Running the complete 7-arm field on **real**
BTCUSDT H1-2024 produces `outcome == AllFragile` on **every** run — including
when buy-and-hold is the top-Sharpe arm — so the honest `BenchmarkWins`
recommendation the product was specced to surface never fires, and the
Robust/Marginal/Fragile discrimination is rendered mute.

Two parallel decision-support notes adjudicated the finding:

- **Analyst** (`spec/dev-notes/robustness-gate-allfragile-analysis-2026-06-22.md`):
  "all ACTIVE arms Fragile" is the **honest, designed-for truth** (the bands are a
  curve-fit detector for *active* strategies, pre-registered 2026-05-30,
  calibrated on daily/4h multi-symbol active θ-surfaces — buy-and-hold was the
  benchmark every one of those surfaces was scored *against*, never a
  robustness-judged candidate). "Hold **also** Fragile ⇒ `AllFragile`" is a
  **category error**: the candidate-overfit ruler pointed at the null hypothesis
  the candidates are measured against. The product already half-encodes the
  distinction (`is_benchmark` + `BenchmarkWins` exist); the single missed
  exemption is `rank.rs`'s `all_fragile` counting the benchmark's own flag.
- **Architect** (`spec/dev-notes/robustness-gate-allfragile-technical-2026-06-22.md`):
  confirms the seam is `rank.rs` (not `classify_verdict`), that the benchmark's
  p5-Sharpe < 0 is the (near-certain) binding signal on a 60-70%-vol single
  asset under 1000-path resampling — a **correct** computation, not a numeric bug
  — and that the fix is **anchor-safe by construction** (the advisor path writes
  no report; the classifier is untouched; `RobustnessMode::default() == Skip`).
  It also corrected the analyst's edit count: the fix is **two** coordinated
  `rank.rs` edits, not one (see D1 + D2).

The operator approved **B1** (benchmark exemption) as the root fix and
**rejected** B2/B3 (loosening / asset-class-tuning the Fragile bands) — B2/B3
would mutate the constants the 18 block-bootstrap θ-surface anchors are
byte-keyed to (≤18 anchor breaks) and forfeit the pre-registration moat. The
correct fix is also the anchor-cheap fix: B1 breaks **zero** anchors.

This ADR **amends** ADR-0059 § D5 (the F2 comparator outcome rule) and ADR-0063
§ D7 (the R4.4 reachability gate, specced but never implemented). It leaves the
classifier freeze (**ADR-0059 § D4 / ADR-0063 § D4**) and the 2026-05-30
pre-registration discipline **byte-unchanged**.

## Decision

### D1 — The benchmark is not a candidate for the `AllFragile` determination

`rank_candidates` (`crates/backtest/src/bakeoff/rank.rs`) computes the
all-fragile determination over **non-benchmark (active) arms only**. The current
`all_fragile` (rank.rs:60-62) ranges over **every** candidate:

```
// CURRENT — counts the benchmark's own flag:
let all_fragile = candidates
    .iter()
    .all(|c| c.robustness == Some(RobustnessFlag::Fragile));
```

becomes, conceptually, `all_active_fragile`:

```
// AMENDED — benchmark excluded from the outcome determination:
let all_active_fragile = candidates
    .iter()
    .filter(|c| !c.is_benchmark)
    .all(|c| c.robustness == Some(RobustnessFlag::Fragile));
```

with the outcome branch (rank.rs:64-70) firing `AllFragile` **iff all ACTIVE
arms are Fragile AND no benchmark gives a crownable result** (i.e. the benchmark
is itself Fragile-and-not-crown-eligible, or absent). The empty-active edge
(only a benchmark, or no active arms) and the no-benchmark edge fall back to the
existing semantics: a field with no benchmark is judged exactly as today (the
bake-off always appends `v0.buyhold` — mod.rs — so the live advisor path always
has a benchmark; the no-benchmark case is a unit-test / library-caller path,
preserved unchanged, see D5's residual rule).

Rationale: the benchmark is the **null hypothesis the candidates are scored
against** (`spec/runbooks/passive-baseline.md`: "the benchmark every robustness
surface is scored against"), never a candidate that must clear the robustness
bar. This is **framed strictly as "benchmark-is-not-a-candidate," NEVER as "we
loosened the gate."** Every active / ensemble arm still faces the identical
frozen `classify_verdict` against the identical frozen bands — this is **not a
threshold relaxation** and moves no band.

### D2 — The benchmark is crown-eligible irrespective of its own robustness flag

`is_eligible` (rank.rs:124-126) returns `true` for `c.is_benchmark` regardless
of its `robustness` flag:

```
// CURRENT:
fn is_eligible(c: &CandidateResult) -> bool {
    c.robustness != Some(RobustnessFlag::Fragile)
}
// AMENDED — the benchmark is always crown-eligible:
fn is_eligible(c: &CandidateResult) -> bool {
    c.is_benchmark || c.robustness != Some(RobustnessFlag::Fragile)
}
```

This is the **second required edit**, and D1 alone is insufficient — it is in
fact a *worse* bug without D2. The comparator's eligibility partition (rank.rs:
92-99) sorts ineligible arms last. Without D2, an all-Fragile-incl-benchmark
field would set `all_active_fragile = true` (all active arms Fragile) **but the
benchmark would still partition as ineligible**, the crown would land on a
Fragile *active* arm, `crowned.is_benchmark` would be `false`, and the outcome
would fall through to `ActiveWins` **on a Fragile active crown** — strictly worse
than today's honest `AllFragile`. Both edits are mandatory; both live entirely
within `rank.rs`; the classifier is untouched.

The active-arm eligibility lock is **unchanged**: an active Fragile arm stays
ineligible to be crowned (the anti-overfit lock holds — ADR-0059 § D5). Only the
benchmark gains unconditional crown-eligibility, because it is the baseline, not
a candidate that must clear the bar.

### D3 — `classify_verdict` + `verdict_bands` are BYTE-UNCHANGED

Reaffirms the **ADR-0059 § D4 / ADR-0063 § D4** classifier freeze. The bootstrap
still runs for the benchmark arm (bootstrap.rs is untouched); its
`RobustnessFlag` is still **computed** and still **displayed** on the leaderboard
row (informational: "the baseline is itself path-dependent on a single volatile
asset"). Only its **consumption** in `rank_candidates` changes — it no longer
disqualifies the field into `AllFragile` (D1) nor gates the benchmark's own
crown-eligibility (D2). The 2026-05-30 pre-registration discipline is intact: **no
band is moved**, no constant in `robustness.rs::verdict_bands` is touched, and no
classifier signal threshold changes. This is the line between B1 (this ADR) and
the rejected B2/B3.

### D4 — Anchor safety by construction (119/119)

`verify_anchors.sh` reads **119/119 before the first seam and after the last**;
any non-119 is a STOP-and-route-back. The proof, traced from `rank.rs` output to
every anchored byte:

- **The advisor bake-off path writes NO report.** `run_bakeoff` constructs every
  `ScenarioConfig` with `write_report = false` (mod.rs, ADR-0059 § D3). The
  `BakeoffReport` / `Recommendation` / `outcome` are in-memory only, consumed by
  the cockpit, never serialized to a `spec/*/reports/` body — so a change to
  `rank_candidates`' `outcome` **cannot** alter any anchored file's body-SHA.
- **`classify_verdict` + `verdict_bands` are byte-untouched (D3).** The 18
  block-bootstrap θ-surface anchors (`v1-basis-reversal-fee*-…`, `v2-mn-*-…`)
  hash a body whose per-θ ROBUST/MARGINAL/FRAGILE verdicts come from the **sweep
  bin** (`bin/param_robustness_sweep.rs` → `classify_verdict`), **not** from
  `rank_candidates`. `rank_candidates` is the advisor comparator and has never
  written an anchored body. The bands the sweep reads are unchanged ⇒ those 18
  surfaces are byte-identical.
- **`RobustnessMode::default() == Skip`** (mod.rs, ADR-0063 § D5). Every anchored
  CLI path keeps `robustness == None`, and `is_benchmark` only matters when a
  benchmark arm is present in a bake-off, which no anchored report path
  constructs. The eligibility path is byte-unchanged on anchored runs.

This is the same additive-equivalent contract as ADR-0059 § D3 (the `v0.buyhold`
arm) and ADR-0063 § D5 (the `RobustnessMode::Bootstrap` activation): the new
behaviour is advisor-path-only; existing anchored bytes are untouched. **No
`anchors.toml` SHA changes; no `data/*/REVISION.toml` is touched; no anchored
report is opened, edited, or re-emitted.** Confirmed 119/119 at HEAD this session.

### D5 — Day-1 reachability gate (implements the missing ADR-0063 § D7 / R4.4)

Ship the FAIL-before / PASS-after reachability e2e — this is **not net-new
scope**, it finally lands the R4.4 reachability regression ADR-0063 § D7
promised but never implemented (`robustness_bootstrap_bites.rs:9` *declares*
"Allow `BenchmarkWins` to remain reachable when all active strategies lose" in a
doc comment, but **no test body asserts it**; grep confirms no
all-active-fragile → `BenchmarkWins` assertion in `crates/backtest/tests/` or
`crates/ui/tests/`). The CLAUDE.md day-1-divergence-test discipline applied to an
outcome-determination change:

- **The reachability assertion (FAIL-before / PASS-after):** a field where **all
  ACTIVE arms are flagged Fragile** AND the **benchmark is the top-Sharpe arm**
  must yield `outcome == BenchmarkWins` (reason `BenchmarkUndefeated`) **AND**
  `candidates[crowned].is_benchmark == true` — **NOT** `AllFragile`. Today this
  field returns `AllFragile`; after D1+D2 it returns `BenchmarkWins`.
- **The residual `AllFragile` dual:** a field where all active arms AND the
  benchmark are Fragile AND the benchmark is NOT crownable-by-Sharpe (an active
  arm out-Sharpes it) still yields `AllFragile`. With D2 the benchmark is
  crown-eligible, so when it does **not** have the top Sharpe the crown is an
  active arm and the field is genuinely all-fragile ⇒ `AllFragile` (reason
  `AllCandidatesFragile`). The no-benchmark single-Fragile-arm case
  (`rank.rs::single_fragile_candidate_is_crowned`) is unaffected and stays
  `AllFragile`.
- **Amend `rank.rs::t65_all_fragile` (rank.rs:297-325).** Its 2-arm fixture
  (`v0.sma` Fragile Sharpe 2.0 + `v0.buyhold` Fragile Sharpe 1.0, benchmark)
  currently asserts `AllFragile` on the benchmark-inclusive `all_fragile` field.
  Under D1+D2 the only ACTIVE arm (`v0.sma`) is Fragile but the benchmark is now
  crown-eligible; `v0.sma` out-Sharpes the benchmark so the **active arm is
  crowned** and the field is genuinely all-fragile ⇒ the corrected expectation
  for *this* fixture is **still `AllFragile`** (active arm crowned, no crownable
  benchmark by Sharpe). To exercise the new `BenchmarkWins` path the developer
  **adds** a sibling case with the benchmark as top-Sharpe (e.g. benchmark Sharpe
  1.0, the lone active arm Fragile at Sharpe 0.5) asserting `BenchmarkWins` +
  `crowned.is_benchmark`. The `t65` amendment is a **deliberate, ADR-logged
  unit-test behaviour clarification**, not a regression — anyone reading the diff
  must read it as the corrected semantics.

> **Edit-shape note for the developer (the seam subtlety the ADR pins).** Both
> the `t65` fixture (benchmark NOT top-Sharpe) and the new reachability case
> (benchmark IS top-Sharpe) are needed because the outcome now depends on
> *whether the crownable arm is the benchmark*, which depends on the Sharpe
> ordering relative to D2's newly-eligible benchmark — a property the old
> benchmark-inclusive `all_fragile` short-circuit hid. Construct
> `CandidateResult`s with explicit flags (the existing `make_candidate` /
> `t65` pattern); no corpus, no bootstrap needed for the pure-`rank_candidates`
> reachability assertion.

### D6 — Determinism unchanged

`rank_candidates` stays pure / total / deterministic. D1 adds a `filter`; D2 adds
one boolean disjunct to a predicate — both pure, total, no new f64 boundary
(only `f64::total_cmp` and `Decimal::cmp`, rank.rs:17), no RNG, no `SystemTime`.
Identical input ⇒ identical `Ranking`. This ADR introduces **no new determinism
boundary**.

## Alternatives considered

- **Edit only `all_fragile` → `all_active_fragile` (D1 without D2)** — rejected:
  a *worse* bug. Without D2 the comparator still partitions a Fragile benchmark as
  ineligible, the crown lands on a Fragile active arm, `crowned.is_benchmark` is
  false, and the outcome falls through to `ActiveWins` on a Fragile crown — see
  D2. Both edits are required.
- **Loosen / asset-class-tune the FRAGILE bands (B2/B3)** — rejected: mutates the
  `verdict_bands` constants the 18 block-bootstrap θ-surface anchors are byte-keyed
  to (≤18 anchor body-SHA breaks + per-anchor ADR-0038 § D6 re-emission) and
  forfeits the 2026-05-30 pre-registration moat (the post-hoc goalpost-move the
  discipline exists to forbid). "AllFragile on crypto" is the rule **correctly**
  reporting single-asset crypto has no robust active edge — not a calibration
  miss. Anchor-mutating + trust-eroding; the correct fix (B1) is also the
  anchor-cheap fix (0 breaks).
- **Null the benchmark's `robustness` flag entirely** — rejected: it is honest to
  *show* that buy-and-hold is itself path-dependent on a single volatile asset
  (D3 keeps it computed + displayed); the fix is to stop letting that fact nuke
  the recommendation into nihilism, not to hide it.
- **Skip the bootstrap for the benchmark arm** — rejected: same honesty reason as
  above, and it would special-case the benchmark in the compute path (bootstrap.rs)
  when the only needed change is in the *consumption* path (rank.rs).

## Consequences

- The advisor regains `BenchmarkWins` ("hold is least-bad; no active arm was
  robust") as the **modal honest crypto outcome** — what the product was specced
  to say (`product.md`: "when buy-and-hold wins the bake-off, the recommendation
  says so"). The nihilist `AllFragile` no longer masks a benchmark that is, in
  fact, the top arm by Sharpe.
- **What breaks if D2 is omitted:** the crown lands on a Fragile active arm and
  the outcome is `ActiveWins` on a Fragile crown (worse than today). Caught by the
  D5 reachability e2e asserting `crowned.is_benchmark`.
- **What breaks if D3 is violated** (any band moved): the 18 block-bootstrap
  θ-surface anchors mutate ⇒ `verify_anchors.sh` < 119 ⇒ REGRESSION. Mechanically
  caught by `scripts/verify_anchors.sh` (run before + after) — the developer gate.
- **Frozen surfaces honoured:** `classify_verdict` + bands (ADR-0059 § D4 /
  ADR-0063 § D4), the active-arm anti-overfit eligibility lock (ADR-0059 § D5,
  benchmark-only change), `Strategy` trait (ADR-0005), the 2026-05-30
  pre-registration. This ADR amends the comparator **outcome** rule (ADR-0059
  § D5) and lands the **reachability** gate (ADR-0063 § D7); it amends the
  classifier freeze in **no** way.
- `cargo tree -p ui` unchanged (the corrected `outcome` reaches the cockpit
  through the existing `BakeoffReport` mirror; no new seam, no new edge).
- This ADR does not add, remove, or mutate any of the 9 anchor SHAs in
  `spec/anchors.toml`; `rank_candidates` produces no anchored artifact.

## Changelog

- 2026-06-22 (architect): initial accept. Benchmark exemption from the
  `AllFragile` outcome determination — the operator-approved B1 root fix for the
  always-`AllFragile`-on-real-crypto finding. D1 `all_fragile` →
  `all_active_fragile` (filter `!is_benchmark` from the outcome determination);
  D2 `is_eligible` returns true for the benchmark regardless of its own flag (the
  required second edit — D1 alone is a worse bug, crowning a Fragile active arm);
  D3 `classify_verdict` + `verdict_bands` BYTE-UNCHANGED (the benchmark's flag
  stays computed + displayed, only its consumption changes — framed
  "benchmark-is-not-a-candidate," NOT a threshold relaxation; no band moved);
  D4 anchor-safe by construction 119/119 (advisor path `write_report=false`,
  classifier frozen, `default()==Skip`; the ≤18 θ-surface anchors come from the
  sweep bin not `rank_candidates`); D5 the day-1 BenchmarkWins-reachability gate
  (all-active-fragile + benchmark top-Sharpe → `BenchmarkWins` + `crowned.is_benchmark`,
  finally implementing the ADR-0063 § D7 / R4.4 promise the
  `robustness_bootstrap_bites.rs:9` doc comment declared but no test body
  asserted) + the `t65_all_fragile` amendment + the residual `AllFragile` dual;
  D6 determinism unchanged (pure/total, no new f64 boundary, no RNG). AMENDS
  ADR-0059 § D5 (comparator outcome rule) + ADR-0063 § D7 (R4.4 reachability);
  leaves the classifier freeze ADR-0059 § D4 / ADR-0063 § D4 + the 2026-05-30
  pre-registration UNCHANGED. REJECTS B2/B3 (band loosening — ≤18 anchor breaks +
  moat forfeiture). Leans on ADR-0059, ADR-0063, ADR-0051. Anchor-mutation ADR
  NOT triggered (no `anchors.toml` SHA change). Feature: `advisor-benchmark-robustness`.
