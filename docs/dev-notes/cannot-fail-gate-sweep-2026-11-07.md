---
slug: cannot-fail-gate-sweep-2026-11-07
status: record
owner: orchestrator
updated: 2026-11-07
---

# Cannot-fail-gate sweep — 2026-11-07

A seven-angle adversarial sweep for gates and tests that cannot fail, run after three new
instances of that shape were found and fixed the same day (`#153` an unreachable derivation,
`#154` a test that re-implements its subject plus a boundary-insensitive fixture, `#155` a
throttled tick destroying its window's count). **61 agents, 7 find angles, 2 adversarial
verification lenses per candidate, 1 completeness critic. 26 candidates: 14 survived, 12 were
refuted or already disclosed.** The critic then found 4 more, one of which is the most serious
finding of the day.

Every finding was required to carry the command that proved it. That constraint was set because
the orchestrator twice that day stated a plausible claim that one available command disproved.

## Read this first: three things the generated report below gets wrong or cannot see

**1. Every one of the seven angles hit its 4-finding cap. 31 further candidates were dropped.**
The report's § 4 says `cap_hit` is "unknown for every angle" and treats one angle as *possibly*
truncated. That is an artefact of the orchestration script, which handed the synthesizer the
findings and verdicts but not the per-angle `what_i_searched` / `cap_hit` / `dropped_note`
fields. Those fields were emitted and are in the run's logs. The real tally:

| angle | reported | dropped at the cap |
|---|---|---|
| boundary-insensitive | 4 | 4 |
| reimplements-subject | 4 | 5 |
| unreachable-derivation | 4 | 6 |
| feature-gated-skips | 3 (+1 known) | 3 |
| vacuous-assertions | 4 | 4 |
| unmeasured-counts | 3 (+1 known) | 4 |
| computed-never-consumed | 4 | 5 |

So **the 14 confirmed findings are a floor, not a census** — and the cap, not the tree, is what
bounded six of the seven angles. The dropped candidates are described in the run log with their
evidence; several are one grep from being findings. Read § 4's "specifically NOT swept" alongside
this table, not instead of it.

**2. The critic's F-A is the headline, not an appendix — and it was independently re-verified
by the orchestrator before this note was written.** See the next section.

**3. The feature-gated-skip angle came back a genuine clean negative.** All four of its
candidates were already disclosed (`#81`, `#87`, `#93`, `#111`, `#121`) or decided by ADR. That
corrects the orchestrator's own framing earlier that day, which had called that angle "the
highest-value, only partly understood" on the strength of one instance
(`forecast_distribution`'s candle gate). Measured, it is the best-covered angle of the seven.

## The headline: 2 of the 4 AD-9 reconciliation identities cannot fail, and four committed bodies publish them as PASS

Filed as bug-log `#156`. **Verified directly, not taken from the agent's report:**

`crates/reports/src/lib.rs:190-191`

    equity_delta:     realized_period.amount() + unrealized,
    equity_check_sum: realized_period.amount() + unrealized,

Two character-identical expressions, assigned to the two fields that `reconcile::compute`
subtracts at `reconcile.rs:119-122`. `delta == Decimal::ZERO` unconditionally, for every input,
forever. Identity 1 is the same defect one hop out: `lib.rs:183` defines
`headline_return_usdt = realized.amount() + unrealized`, passes it and both summands separately,
and `reconcile.rs:104-106` compares the one against the sum of the other two.

And the row's own label names a term that exists nowhere in the computation:

    $ grep -rn 'fees_delta' crates/ --include='*.rs'
    crates/reports/src/reconcile.rs:9    (doc comment)
    crates/reports/src/reconcile.rs:66   (doc comment)
    crates/reports/src/reconcile.rs:92   (doc comment)
    crates/reports/src/reconcile.rs:119  (the row label)

So the published row advertises a three-term identity while the code checks a two-term tautology,
and the missing term is **fees** — on the invariant (AD-9) that exists to guarantee exact-cent
reconciliation in a double-entry ledger.

**The shipped consequence.** Four committed operator-facing bodies publish a four-row
Reconciliation table, all PASS, two of them tautologies:

    | Identity                                          | Report | Ledger | Δ | Pass? |
    | headline_return = realized + unrealized           | 0 | 0 | 0 | PASS |   <- cannot fail
    | Σ pnl_by_strategy = Σ realized                    | 0 | 0 | 0 | PASS |
    | Σ pnl_by_symbol = Σ realized                      | 0 | 0 | 0 | PASS |
    | equity_delta = realized + unrealized + fees_delta | 0 | 0 | 0 | PASS |   <- cannot fail

`evidence/v1/operator-success-reports/reports/success-fixed-report-sample-{7d,90d}.md` and
`evidence/v1/paper-soak-longevity/reports/operator-success-{soak,research-replay}-2026-06-19.md`.

Only identities 2 and 3 are independently sourced — and those are precisely the two the single
FAIL-path test breaks (`crates/reports/tests/reconciliation_mismatch.rs:4-11`). Every fixture in
the tree builds `ReconciliationInputs` by hand, so they all test `compute`'s arithmetic and none
tests the wiring that feeds it.

**Anchor consequence, and the reason this needs an operator ruling.** Two of the four bodies are
anchored: `anchors.toml` entries `report-sample-7d` and `report-sample-90d` resolve to
`success-*-<scenario>.md` under `evidence/v1/operator-success-reports/reports/`. A filename grep
of `anchors.toml` returns zero hits for all four files — the documented false negative, since
anchors are keyed by scenario NAME. Correcting the published tables therefore moves two anchored
body-SHAs and requires the ADR-0038 § D6.b re-emission protocol.

**And the fix is not mechanical.** Making identity 4 real needs an equity snapshot pair and a
fees total at that call site. If those are not available there, the honest alternative is to stop
publishing the row as PASS and publish it as unmeasured — the house rule is that a skip is
unmeasured, never a pass. Either path changes what the project publishes.

## Disposition

**Needs an operator ruling** (touches `evidence/`, or changes what is published):
- `#156` / F-A — the reconciliation tautology: the code fix, and whether the two anchored bodies
  are re-emitted under § D6.b or the row is republished as unmeasured.
- Ranked action 3's second half — making the field-invariance report's hardcoded `✓` table a real
  comparison moves the two `recalibrate-sigma-train-bs{1,2}` body-SHAs.

**No ruling needed, test-only, no anchor impact** — ranked actions 1, 2, 4, 5 and the F11–F14
hygiene pass. Highest value first: F1 (the AD-19 redactor's single maximum-entropy fixture) and
F2 (four of five AD-1 ROBUST bands widenable into no-ops with 368 tests green, `classify_verdict`
itself untouched).

### Closed since this note was written

- **`#156` / F-A — CLOSED 2026-11-07/08.** The operator ruled "build the real identity" + "make
  the equity curve ledger-derived"; both landed, and the D6.b re-emission landed with them
  (`cd14098c`, `23374613`). It took **three** captures — the two discarded ones became `#158`
  (the anchors had been clock-independent only because the curve was a constant; the first
  capture would have gone red the next morning) and `#159` (the replacement identity asserted
  `unrealized`, which **nothing in the repo ever posts to the journal** — so it passed on the
  two anchored bodies for the same accidental reason the tautology had, both fixtures having no
  open positions). `#157` records that `verify_anchors.sh` resolves bodies by byte order over
  full paths, so the obvious re-emission filename would have left the gate hashing the
  superseded body. Record: [`156-d6b-re-emission-2026-11-07.md`](156-d6b-re-emission-2026-11-07.md).
  The two unanchored soak bodies are **not** re-emitted and cannot be — their ledger is gone.
- **F12, F13, F14 — CLOSED 2026-11-08 as `#174` and `#175`.** The whole below-the-line tier is
  done. F12's counters joined the existing `tracing::info!`; ADR-0078's return-vol-ρ Consequences
  bullet is corrected (no reader, so no operator ever saw one — the tense was the false part).
  F13's two fields are KEPT with corrected docs, not deleted: unlike `#162`'s they are correct,
  merely idle. F14's zero-assertion test now proves its channel is live before asserting a zero,
  and four false sentences across three module docs are gone.
- **F3 — CLOSED 2026-11-08 as `#173`** (ranked action 3, first half). One in-crate test drives
  the real private `WindowIterator`. The demonstration this finding deserved: all three
  perturbations of the shipped formula leave the ADR-named gate **4/4 green** and redden the new
  one. Three artefacts corrected — and the ADR's own sentence turned out to be ACCURATE; listing
  it under "Enforced by" was the defect, not the wording.
- **F9 — CLOSED 2026-11-08 as `#171`** (ranked action 5, last third). Seven two-sided tests,
  each asserting its driving metric's exact value first. **This note's L1 numbers were wrong:**
  0.97 is green and 0.9701 reddens, so the interval is (0.95, 0.97] not (0.95, 0.98) — it tested
  {0.96, 0.98} and interpolated. Probed nine ways including all three values it called green.
  Its bonus observation became `#168`.
- **F6 — CLOSED 2026-11-08 as `#170`.** `t_classifier` lifted into `backtest::stats` (anchors
  119/119 unmoved — pure fn, no state), both bands pinned two-sided in the lib, the IEEE `-0.0`
  trap stated, a three-distinct-labels floor, and a band-prose gate reading the bin via
  `include_str!`. **This note's own fix for F6 was inapplicable** and is superseded: two
  bin-local tests would never have run in CI (see the `#163` entry above). Probed 8 ways
  including this note's 1.0 and the `-1e9` collapse.
- **F8 — CLOSED 2026-11-08 as `#169`** (ranked action 5, first third). Two-sided pair reaching
  the shipped predicate, the three worked examples reading the constant, and a non-vacuity count
  on the downward half. Probed both ways including this note's own 1.1: 3 failed where the whole
  suite was green. Interval (0.7804, +∞) → (0.949630, 0.950499]. DSR stays report-only.
- **F4 — Parts A+B CLOSED 2026-11-08 as `#167`; Part C operator-ruled DOCUMENT-NOT-RE-EMIT.**
  The gate now calls the shipped substitution (`forecast::provenance::overlay_with_sigma_train`)
  and asserts the two committed checkpoint pairs behind a floor. This finding's `measured_evidence`
  was right about the `✓` and about the ULP's cause, and **wrong that the two values are
  "identical as a value"**: `json.loads` and `str::parse::<f64>` both give two distinct f64s;
  only serde_json collapses them, because its float parser is off by one ULP on that literal.
  That is why the first replacement test passed for the wrong reason, and why a third,
  text-level test now declares the BS-2 artefact explicitly. Deviation record:
  [`recalibrate-field-invariance-deviation-2026-11-08.md`](recalibrate-field-invariance-deviation-2026-11-08.md).
- **F10, F7, F5 — CLOSED 2026-11-08 as `#164`, `#165`, `#166`** (ranked action 4). All three
  got a floor that refuses to report on an empty set plus a count in the output. **F5's stated
  fix in this note was a no-op:** the gate is blind in TWO independent places (capture AND
  story-file resolution) and the note named one — verified by measuring exit 0 with zero output
  after rewriting a backlog line to be captured. This note's "13-case `--self-test`" for F5 is
  also 11 fixtures / 14 assertions. F10's floor was checked against all 18 committed decks and
  the 14 non-zeros are not regressions — see `#164` for why that population is the wrong one.
- **F6 — superseded in scope by `#163`.** Its prescription (two bin-local tests) was
  **inapplicable**: measured, `cargo test -p backtest --bins` silently skips
  `src/bin/threshold_sweep.rs` entirely, so the 6 tests already in that module have never run
  in CI either and a 7th and 8th would join them. That measurement generalised into `#163` —
  46 test fns across 13 targets invisible to CI — and the remaining F6 work (lift `t_classifier`
  into `stats/` where CI reaches it, then two-sided pairs) is still open.
- **F11 — CLOSED 2026-11-08 as `#162`.** The field is deleted. **Two of this finding's own
  supporting claims were false** and are corrected in `#162`: `crates/backtest` *does* have a
  `reports` dependency (a documented acyclic dev edge), and `reports::generate` *does* render
  anchored files — 4 anchor rows resolve to its output. AD-2 held anyway, because the wrong
  digest had no reader. The finding's real yield was elsewhere: the Rust/python convention
  agreement was gated by a doc comment, in the one crate that renders anchored bodies, and is
  now a test that execs the canonical hasher.
- **F2 — CLOSED 2026-11-08 as `#161`** (ranked action 2). Every ROBUST edge now pinned from
  both sides: exactly-at the band → ROBUST (catches narrowing, pins the inclusive comparison),
  just-outside with the other four held inside → MARGINAL (catches widening), plus
  `classifier_robust_does_not_collapse_to_p5_alone`. Probed: all four widened at once → 2
  failed; each individually → 1 failed; the narrowing control → 1 failed; control 7 passed.
  `classify_verdict` and `verdict_bands` stayed byte-frozen — verified by diffing both against
  HEAD, and by the diff being a single pure insertion inside `mod tests`.
- **F1 — CLOSED 2026-11-08 as `#160`** (ranked action 1). Seven fixtures whose entropies are
  *derived* from a closed form (`H = log2(n) - 2a/n`) rather than measured, straddling 4.5 at
  both length 32 and length 40, plus a 31-vs-32-byte pair closing the `MIN_LEN` headroom;
  `entropy_high_for_mixed`'s self-referential assertion replaced with an absolute value. Probed
  in both directions on both constants: the two values this note recorded as **silently green**
  now fail — `ENTROPY_THRESHOLD = 5.12` → 3 failed, `ENTROPY_MIN_LEN = 35` → 2 failed, control
  112 passed / 0 failed. The first probe was itself broken (a shell loop split its patterns on
  `:`, so all four perturbations ran unmodified code and reported `0 FAILED`); the rewrite
  refuses to report a result unless the file differs from its snapshot — a corollary for the
  house rule, since a probe that silently no-ops is a gate that cannot fail one level up.

**Correctly out of scope:** do not wire DSR as a crown veto (do-not-build E-1, ADR-0075 D2) and
do not move any threshold — every fix here is a fixture or a floor, never a band.

## The generated report, verbatim

# Cannot-Fail-Gate Sweep — 2026-11-07

## 1. Verdict

Seven angles were swept and the tree is **not** in better shape than the day's three findings suggested: 14 candidates survived both adversarial lenses, and the two strongest sit on non-negotiables — the AD-19 secret redactor (whose only positive fixture is a string at the arithmetic *maximum* of the metric it gates, so the threshold can be raised past a real 40-char AWS secret with all 31 tests green, demonstrated by running the shipped `apply_redaction_for_field`) and the AD-1 byte-frozen robustness classifier (four of its five ROBUST criteria can be widened into no-ops simultaneously with 368 shipped tests green, both controls proving the harness bites on the directions that *are* covered). The dominant shape by count is the boundary-insensitive fixture — the same sub-shape #154 filed and fixed this morning for one classifier, now measured in four more places that #154's census and ADR-0099's gate do not reach. Two gates were found to contain no code from their subject at all (`parkinson_target_derivation.rs` compiles and passes under `rustc --test` with no `forecast` crate in scope; `recalibrate_sigma_train_field_invariance.rs` links zero `write_overlay` symbols), and three `scripts/` gates report clean while reconciling an empty set. Offsetting that: the compiled-out/skip-as-pass angle came back a **clean negative** — all four of its candidates were either already disclosed (#81, #93, #111, #121, audit §6) or decided by ADR, and nine further candidates were refuted outright or shown deliberate-and-documented, including three where the reporter's own supporting claim was false. Severity is mostly latent rather than live: the live corpus measures 784 FRAGILE / 80 MARGINAL / 0 ROBUST with max p5 = 0.18, so the frozen-band gap relabels nothing today, and no finding moved a published number.

## 2. Confirmed findings

### F1 — The secret-redaction entropy gate's only positive fixture is a maximum-entropy string
`crates/llm/src/redact_layer.rs:121` (`ENTROPY_THRESHOLD = 4.5`), `:122` (`ENTROPY_MIN_LEN = 32`), rule at `:335`.
**Shape:** boundary-insensitive fixture, stacked on expectation-from-the-measurement.

Green under `ENTROPY_THRESHOLD` anywhere in **(0, 5.129283]**; first redness at 5.2. `ENTROPY_MIN_LEN` 32→35 is green; first redness at 36 (three units of silent headroom, not "zero margin"). The only firing fixture, `aB3#kP9@mZ2$nQ7&rX4!sY1*tW6^vU0_wV5`, is 35 bytes all distinct → H = log2(35) = 5.129283, the arithmetic maximum for its length. Exactly two assertions can catch a raise and both use that one string; the second (`entropy_high_for_mixed`, `:893`) is `assert!(shannon_entropy(FIXTURE) >= ENTROPY_THRESHOLD)` — its expectation *is* the shipped constant, so it can never pin an absolute threshold. `grep -rn "entropy|ENTROPY" --include=*.rs .` outside this file returns nothing; the end-to-end consumer test carries only `sk-ant-` values caught by regex rule 0.

Consequence probe (real `apply_redaction_for_field`, suite 31/31 green at 5.12): fields whose only matching fragment is `key` — `signing_key`, `private_key` — carry the textbook 40-char AWS secret and flip to `redacted=false rule=<NONE>`, because regex rule 6 matches but is `continue`d on field-name context; and *any* key/token/secret field with a 32–39-char value flips too (rule 6's `{40}` cannot match, and log2(32) = 5.0 < 5.12, so the rule becomes provably unreachable at length 32). `secret_key`/`access_token` still redact via `aws_secret_context`.

```
sed -i '' 's/^const ENTROPY_THRESHOLD: f64 = 4.5;/= 5.12;/' src/redact_layer.rs && cargo test --offline
# 30 passed; 0 failed; 1 ignored   (control at 5.2: 2 FAILED)
diff <(cargo test --offline -- --list) <(cd $REPO && cargo test -p llm --lib -- --list)  # IDENTICAL, 31 tests
```
**Smallest fix:** four fixtures — a value just below and just above 4.5 at length 32 and at length 40, asserted not-redacted / redacted — and replace `entropy_high_for_mixed` with an absolute-value assertion. Also `:798` labels the 35-char fixture "// 36 chars".

### F2 — Four of five ROBUST bands in the AD-1 frozen classifier have no detecting fixture
`crates/backtest/src/bakeoff/robustness.rs:102,104,106,108` (used `:123-124,:146-149`).
**Shape:** boundary-insensitive fixtures. The function is byte-frozen by AD-1 and must not be edited — the gap is in `mod tests`.

`P50_SHARPE_ROBUST` 1.0→0.0, `PROB_LOSS_ROBUST` 0.15→0.99, `PROB_SHARPE_GT1_ROBUST` 0.60→0.0 and `P95_MAXDD_ROBUST` 0.50→0.99 — **all four at once**, reducing ROBUST to "not FRAGILE and p5 ≥ 0.5" — leaves **241 lib + 43 bin + 84 integration = 368** shipped tests green. The four widenings are provably unconditional given non-FRAGILE. Only the FRAGILE five and `P5_SHARPE_ROBUST` are two-sided; the other four are pinned on exactly one side (the "just clearing" side fixed by `classifier_robust_boundary_all_just_above`) and unbounded on the widening side, because all four Marginal fixtures (p5 = 0.0, 0.3, 0.3, 0.3) are Marginal via p5 < 0.5 alone. Every unpinned direction labels *more* candidates ROBUST.

Nothing else can catch it: the constants are referenced nowhere outside the file; `scripts/` and `.githooks/` contain zero references; `verdict_transcription_check.py` covers only the three forecast classifiers; `rank.rs:70,74,152` treats Marginal and Robust identically (only `Some(Fragile)` is ineligible), so the AD-1 ranking-identity tests cannot see it; `theta_surface_reproduction.rs` — the one gate that re-runs verdict-bearing bodies — is `#![cfg(feature="realdata")]` + `#[ignore]`. Blast radius is label-only: `crates/ui/src/leaderboard/state.rs:833`, `crates/agent/src/narration.rs:162`. **Latent, not live:** anchored per-cell verdicts are 784 FRAGILE / 80 MARGINAL / 0 ROBUST and max p5 on any MARGINAL cell is 0.182983.

```
rsync -a --exclude target/ --exclude .git/ ./ /tmp/copy/   # repo untouched; sha matches
sed -i '' -e 's/P50_SHARPE_ROBUST: f64 = 1.0;/= 0.0;/' … (4 constants)
cargo test --offline -p backtest --lib --bins   # 241 passed, 43 passed
cargo test --offline -p backtest --test robustness_bootstrap_bites … (11 targets)  # 84 passed
# CONTROL A P5_SHARPE_ROBUST 0.5->0.3 -> marginal_p5_in_between FAILED
# CONTROL B P50 0.0->1.05 (pinned side) -> classifier_robust_boundary_all_just_above FAILED
```
**Smallest fix:** four two-sided fixture pairs, one per ROBUST edge, with the other four criteria held clear by construction and each first asserting its fixture lands on the intended value — the pattern #154 landed for V4. The function stays byte-frozen; only `mod tests` grows.

### F3 — The ADR-0038 §D3 / T-AR-3 Parkinson gate contains no production code
`crates/forecast/tests/parkinson_target_derivation.rs:47,:54` (test-local helpers); subject at `crates/forecast/src/features.rs:778-804` (formula `:797`).
**Shape:** re-implements-subject, over an arm no test executes.

The entire test file compiles and passes under `rustc --test --edition 2024` with **no `--extern` and no `forecast` crate in scope** (EXIT=0, 4 passed) — complete proof that the binary contains nothing from `features.rs`, so no edit to the shipped constant, the per-bar `high>0 && low>0 && high>=low` guard, the `/h` divisor or the `Option<f32>` wiring can redden it. `required-features = []`, so it sits in the default `cargo test -p forecast` set and reads as live coverage. The arm is executed nowhere: `Some(VolTargetKind::Parkinson)` occurs exactly once in the tree (production `vol_verdict.rs:761`); every other `vol_target_kind` is `None`, including inside `features.rs`'s own `#[cfg(test)]` module. The copy has **already drifted** — `parkinson_sigma_single_bar`, used by 3 of the 4 tests, carries no guard at all while the shipped arm guards every bar.

```
cp crates/forecast/tests/parkinson_target_derivation.rs $S/pk.rs
cd $S && rustc --test --edition 2024 pk.rs -o pk_standalone && ./pk_standalone   # 4 passed
grep -rn 'vol_target_kind:' crates/   # one Some(Parkinson), in a [[bin]]; all test paths None
```
`verify_anchors.sh` contains no `cargo` invocation, so the anchor corpus cannot see a formula change until a manual re-emission. The `strategy` crate's second Parkinson implementation (`vol_meanreversion.rs:138`) *is* gated by three running tests — so do not claim "nothing tests Parkinson"; the forecast implementation is the uncovered one.
**Smallest fix:** one in-crate `#[cfg(test)]` test in `features.rs` driving the real arm — `WindowIterator::new` (`:650`) takes `Result<Vec<RawBar>, FeatureError>` directly, so no parquet is needed, which retires the file header's stated reason for the copy. Then correct the three artifacts that cite this target as the gate (ADR-0038 ~`:726`, `trace.toml:533`, `retired-surface-inventory-2026-05-22.md:62`).

### F4 — The ADR-0035 D2 field-invariance invariant has no measuring instrument at any of three layers
`crates/forecast/tests/recalibrate_sigma_train_field_invariance.rs:72` (`apply_overlay`, test-local); subject `crates/forecast/src/bin/recalibrate_sigma_train.rs:225` (`write_overlay`); the false table at `:389-455`.
**Shape:** re-implements-subject, plus a claim that cannot be false inside a hashed body.

Nothing in the tree ever compares a produced overlay to its original. (i) `nm -a` over the built test binary finds **zero** `write_overlay` symbols and does find the test's own `apply_overlay`; mutating only the local helper to drop `weights_sha256` turns 2 of 3 red, so the verdict is a pure function of the five-line copy. (ii) The two gates that *do* execute the bin — `anchored_report_reproduction.rs::recalibrate_sigma_train_bs{1,2}` and `recalibrate_sigma_train_readonly.rs::body_is_anchor_dir_invariant` — write the overlay into a tempdir and read only the `.md` back (`run_gate` hashes the markdown); all are `#[ignore]`d, and the readonly target is `required-features=["candle"]` and absent from `cargo test -p forecast --no-run`'s executable list. (iii) The report's own `## Field invariance` rows are literal strings — `(verbatim copy)`, the *same* variable printed in both columns, and a hardcoded `✓` — so the anchored bodies certify a comparison that was never performed.

```
cargo test -p forecast --test recalibrate_sigma_train_field_invariance   # 4 passed
nm -a target/debug/deps/recalibrate_sigma_train_field_invariance-* | grep -ci write_overlay   # 0
cargo test -p forecast --bin recalibrate_sigma_train -- --list   # requires the features: candle
```
Residual true statement worth filing with it: ADR-0035 D2's word "byte-identical" is false for BS-2 (`final_val_loss` …069431 → …069433, a 1-ULP decimal) while "value-identical" is true — replaying the shipped body reproduces both committed overlays byte-for-byte — and no gate can tell the two readings apart.
**Smallest fix:** move `write_overlay`'s four lines into `forecast::provenance` (its `canonicalise` is already in the lib and needs no `candle` — verified by linking it from a default-features probe) so the existing test calls the shipped function. ⚠ Making the invariance table a real key-set comparison **moves the two `recalibrate-sigma-train-bs{1,2}` anchored body-SHAs** → operator ruling required under ADR-0038 §D6.b.

### F5 — `queue_staleness_check.py` reconciles 0 of its 4 reconcilable Queue entries and reports clean in silence
`scripts/queue_staleness_check.py:70-72` (the two marker regexes), the `status is None → continue`, and the silent `return 0`.
**Shape:** unmeasured count — no slug count, no coverage line, no non-vacuity floor.

With the committed backlog **byte-unchanged** and all 156 story files rewritten to `Status: done` — the condition the gate exists to catch, maximally present — it prints nothing and exits 0. Paired arm: changing ONE line so the feature slug sits inside the `` (`…`) `` marker makes the same gate emit "1 drift detected" and exit 1. Live extraction is `['in-progress','backlog','backlog','arch-done','ready-for-dev']` — the parenthesised sprint-status words, 0 resolving to a story — while all four real story ids cited in the Queue exist as files and would resolve if captured. `grep -c "/feature\.md)"` on the backlog is 0: the `(slug/feature.md)` matcher died with `spec/`. The 13-case `--self-test` reports "all cases PASS" because every fixture writes the pre-migration `- **feat-b feature** (`feat-b`).` markup — the gate was re-founded in the same phase that broke it, and only the story side of its fixtures was updated.

```
python3 scripts/queue_staleness_check.py --backlog _bmad-output/planning-artifacts/backlog.md \
  --stories-dir $S/stories-all-done   # EXIT=0, zero bytes of output
```
Scope: advisory orchestrator pre-flight, not CI (`grep -rn queue_staleness .github/ .githooks/` is empty), and no live drift is concealed today — all four story-backed entries sit at non-shipped statuses. The lost capability is the defect.
**Smallest fix:** one regex for the committed `` story `6-9-cockpit-cross-platform` (`in-progress`) `` form, plus a printed "N of M Queue entries reconciled" and a floor that fails at N = 0. Update the self-test fixtures to the post-migration markup in the same pass.

### F6 — `t_classifier`, the T-verdict written into a hashed body, has no test and an unbuildable target
`crates/backtest/src/bin/threshold_sweep.rs:231` (`>= 0.10` T-ALPHA-UNLOCKED, `>= 0.0` T-MARGINAL); production calls `:378` (into the body) and `:1130`.
**Shape:** absent test + required-features invisibility + boundary-insensitive anchored fixtures.

`scripts/callers.sh t_classifier` → 2 call sites, both production, 0 grep-only; `sed -n '1147,$p' | grep -c t_classifier` → 0, so neither `#[cfg(test)]` module touches it; `cargo test -p backtest --bin threshold_sweep` is refused at target selection (`requires the features: candle, realdata`), and nothing in the workspace enables them (`cargo tree -e features -i backtest --workspace` → `default` only). The two gates that re-run the bin (`anchored_report_reproduction.rs::threshold_sweep_bs{1,2}_reproduces_anchor`) are `#[ignore]`d at 255 s / 224 s, outside CI's `cargo test --workspace --exclude ui`, **and boundary-insensitive**: both anchored rows carry Sharpe delta `+1.155769` and `+1.086583`, so loosening `>= 0.10` to anything below +1.086583 leaves both bodies byte-identical, and the `>= 0.0` edge is exercised by no anchored row at all. `verify_anchors.sh` hashes stored files and re-runs nothing.

```
cargo test -p backtest --features candle,realdata --bin threshold_sweep -- --list
# 6 tests: gate_survivor_parser_tests::* and report_stamp_tests::* only
```
The label is advisory and the lane retired, but its predecessor flipped once under re-emission (`docs/dev-notes/129-threshold-sweep-measurement-2026-09-27.md:44-48`) and had to be escalated — the number that triggers that escalation is unprotected.
**Smallest fix:** two bin-local tests, one each side of both edges, in the `#[cfg(test)]` module that already exists at `:1148` — the sibling `TClassifier::classify` in `sharpe_comparison.rs` is pinned from both sides in a bin with no required-features. Separately, the band cites `D-AR-1.i`, an identifier that occurs exactly once in the whole repo (that comment): define it or drop the citation.

### F7 — `operator_ledger_check.py` drops its entire row set on a heading edit and exits 0 silently
`scripts/operator_ledger_check.py:83` (`_HEADING_STRIP_RE`), `:197` lookup, `:202-203` `continue`, `:782` silent `return 0`; unimplemented contract at `:18`.
**Shape:** unmeasured count.

Same planted bad row (FAILED, 524 days stale, no dev-note cite): `## Pending recipes` → 2 issues, exit 1; `## Pending recipes - operator queue` → exit 0, no output; `### Pending recipes` (level demotion) → exit 0 too. Case and a trailing parenthetical are tolerated by design (`## PENDING RECIPES` and `## Pending recipes (operator queue)` both still fire), so the precise trigger is *any heading-text change other than case or a trailing parenthetical, or any heading-level change*. One single point of failure for all three tables, including the only populated one: a Done row with an empty Completed cell fires `missing-completion-date`, and renaming `## Done recipes (audit trail)` to `- audit trail` silences it. On the committed ledger the file as-is and the file with the Pending heading renamed produce byte-identical output — 0 bytes stdout, 0 bytes stderr, exit 0 both — so no observer can distinguish them. The script's own docstring promises exit ≥ 2 for a "structurally unparseable ledger"; `return 2` exists only for a missing file, an OSError and a bad `--today`. The 8-case `--self-test` cannot catch it: every fixture comes from `_make_ledger`, which hardcodes the three canonical headings.

```
python3 scripts/operator_ledger_check.py --ledger $S/A.md --today 2026-11-07   # 2 issues, exit 1
python3 scripts/operator_ledger_check.py --ledger $S/B.md --today 2026-11-07   # exit 0, silent
diff $S/A.md $S/B.md   # 29c29, one heading line
```
**Smallest fix:** a required-table assertion raising exit 2 (making the docstring true), plus a parsed-row count on stdout — the same pattern already ratified for #128b. The wiring half (not in CI or any hook) is already on the record as audit finding F-A across five audits.

### F8 — `DSR_THRESHOLD` is unpinned upward above ~0.7804: no fixture puts the flag on its true side as computed
`crates/backtest/src/bakeoff/scorecard.rs:106`, predicate `:748`.
**Shape:** boundary-insensitive / missing positive fixture. One-sided.

0.95 → **1.1** makes `crown_clears_dsr` a constant `false` (a deflated Sharpe is a probability in [0,1]) and leaves 26/26 scorecard unit tests **and** 3/3 `null_data_no_crown` green. Downward is pinned empirically: `null_data_no_crown.rs:685` asserts `!crown_clears_dsr` for every ActiveWins crown on the GBM/GARCH true nulls, and that branch is reached deterministically on 3 of 10 seeds with DSR 0.3077 / 0.6506 / 0.7804 — so any threshold ≤ 0.7804 fails 1–2 of its 3 tests (measured at 0.0 and 0.78), and that test runs in CI (`ci.yml:202`). **Undetected interval: (0.7804, +∞).** Nothing asserts the flag true as *computed*: `:1167` fires on the degenerate short-circuit at `:714` that never evaluates `:748`; `:1291` is a projection identity; the three DSR-value tests re-derive the formula inline and compare to a literal 0.95; the only true-side fixture in the tree (`crates/ui/src/fixtures.rs:2174-2176`) hand-sets `deflated_sharpe = 0.97` and `crown_clears_dsr = true` as an author-written pair. So the user-facing "Beats holding after the search?" row (`crates/ui/src/strings.rs:3566`) can be wired to a constant "Not yet — edge uncertain" with the suite green.

```
NULL_GATE_DEBUG=1 cargo test -p backtest --test null_data_no_crown -- --nocapture
# dsr=0.3077 / 0.7804 / 0.6506, clears_dsr=false  -> 3 passed
# at 1.1: lib 26 passed, integration 3 passed   |  at 0.0 and 0.78: integration FAILED
```
**Smallest fix:** one `assert!(sc.crown_clears_dsr)` on a `compute_scorecard` input whose DSR genuinely clears 0.95 (no current fixture does — the single-candidate fixture lands near 0.75), plus make the three worked-example tests read `DSR_THRESHOLD` rather than a literal. Do **not** wire DSR as a crown veto (do-not-build E-1; ADR-0075 D2 declined a fixed cutoff). Related: `fdr_annex.rs:55 ALPHA = 0.05` asserts its coupling to `DSR_THRESHOLD` in prose only (`:52-54`, ADR-0098 D2), so an upward move silently falsifies the annex's stated alpha.

### F9 — Three of the four movable literals in `classify_l` have no fixture that detects a move
`crates/trader/src/llm_forecaster/verdict.rs:215` (L1 `>= 0.95`), `:247` (L3 `> 2.0`), `:268` (L4 `> 0.50`).
**Shape:** boundary-insensitive. Only L2 is boundary-pinned.

Measured green intervals with all 17 inline + 20 integration tests passing: **L4 = [0.0, 0.59] closed at both ends** — the threshold can be collapsed to "any single short or duplicate trace makes the window degenerate", inverting the documented intent, and the 0.59 endpoint is green only because `1.0 − 41.0/100.0` is `0.5900000000000001` (0.5901 reddens; the honest witness is 0.58). **L3 ≈ (1.0, 2.5], closed at the top**: `> 2.49` is green (a 2.4× cost overrun stops being flagged) and so is `> 2.5` (the fixture's ratio is 2.5000000000000018), while `> 1.0` *fails* 4 tests (the healthy fixture's ratio is 1.0000000000000007) and `> 3.0` reddens. **L1** `>= 0.95` → `>= 0.96` is green (fixtures 0.97 / 0.94); only 0.98 reddens, so the "control" in the original claim proved nothing about 0.95. L2 `< 0.05` → `< 0.0499` is green, `< 0.03` reddens.

```
# byte-identical carve-out (diff silent; verdict.rs's only `use` is std::collections::HashSet)
for X in 0.0 0.04 0.58 0.59 0.5901 0.60; do sed -i '' "s/> 0.50 || duplicate_frac > 0.50/> $X || duplicate_frac > $X/" …; cargo test --offline; done
```
ADR-0039's "What breaks if this is violated" names the L1 drift hazard by name; its "Enforced by" list answers it with nothing.
**Smallest fix:** adjacent positive/negative pairs for L1, L3 and L4, the way `l2_positive_near_threshold` / `l2_negative_at_threshold` already do it. Lead to check, not proven here: the thresholds are also hardcoded as report text (`bin/llm_verdict.rs:276,280,284` and the L3/L4 `format!` strings), so a move would leave the rendered report asserting the old threshold.

### F10 — `check_presentation.sh` affirms a property of a set it never proves non-empty
`scripts/check_presentation.sh:49` (collect), `:51` (branch on non-empty only), `:59` (the PASS string).
**Shape:** unmeasured count.

A deck with no approval block — and a deck with the `## Approval` heading but the boxes removed — both print `PRESENTATION CHECK PASS (… — approval block UN-ticked)` and exit 0, an affirmative statement about an object not in the file. Proven on two tracked files, not fixtures: both `evidence/v2/*/presentations/ERRATA-2026-07-27.md` contain zero occurrences of "approv" and pass with that exact string. Zero-found and all-clean are byte-identical output. It is the only mechanical check (`spec_lint.py` has no rule, `verdict_transcription_check.py` zero hits, never invoked in `ci.yml` or `.githooks/*`, no self-test), and `present-results` SKILL.md:89-99 makes its quoted PASS the sole precondition for emitting `PRESENTATION → READY`. `doc/structure.md:461` documents it as the "deck-**completeness** check"; completeness is in its written role and it never checks it.

```
bash scripts/check_presentation.sh evidence/v2/advisor-overfitting-scorecard/presentations/ERRATA-2026-07-27.md
# PRESENTATION CHECK PASS (… — approval block UN-ticked)   EXIT=0   (grep -c -i approv → 0)
```
The bold / indented / `*`-bullet pre-tick variants are mechanically confirmed but **hypothetical** — the template emits one form (SKILL.md:165-167), all 15 real decks use it, and the gate catches it (11 of 17 decks correctly fail today because the operator ticks after approving). They are not part of the finding.
**Smallest fix:** count `^[-*] \[[ xX]\]` lines near an approval keyword, exit 2 when the count is 0, and print the count in the PASS string — distinguishing "no approval block" from "operator-ticked block" so the gate stays runnable per-deck.

### F11 — `ReportArtifacts.body_sha256` is read by nothing *and* disagrees with the canonical hasher by one byte
`crates/reports/src/render/mod.rs:39` (doc at `:29-30`), assigned `crates/reports/src/lib.rs:494`.
**Shape:** computed, returned, never consumed.

`grep -rn "\.body_sha256" --include='*.rs'` exits 1 with no output; `scripts/callers.sh body_sha256` returns 9 call sites, every one a call to an unrelated *local* fn of the same name. The read is exhaustively impossible: one declaration, one construction site, no destructuring and no `..base` (`grep "Recommendation"`-equivalent check on `ReportArtifacts {`), derives only `Debug, Clone` so no serde reader, and no site formats the struct wholesale with `{:?}`. Both production callers print only `run_id` / `markdown_path` (`bin/report.rs:86`, `agent/src/cron.rs:116`). The sharper half: the producer hashes the in-memory `body` (post `---\n\n`) while `scripts/hash_report.py` and its Rust mirror strip `^---\n.*?\n---\n`, leaving the blank line in — on a real anchored report the two digests are `f6f971ac…` vs `a5b60e4c…`, one leading newline apart, since v1, with 119/119 green.

AD-2 is **not** weakened and must not be claimed: `crates/backtest` has no `reports` dependency, so `reports::generate` writes none of the 119 anchored files; `verify_anchors.sh` execs `hash_report.py` rather than being a second definition; and the producer's fence is pinned green by `t807_front_matter_starts_and_ends_with_fence`.
**Smallest fix:** delete the field and the false doc sentence, or add one assertion in `determinism.rs` comparing the returned value to the re-derived one — which forces the one-byte boundary question to be settled instead of inherited.

### F12 — Two vol-overlay suppression counters and the whole `return_vol_correlation` map have zero consumers
`crates/strategy/src/vol_targeting_overlay.rs:456,:458` (incremented `:704,:707`); `return_vol_correlation` same file.
**Shape:** computed, never consumed.

Both counters are read nowhere outside the defining file (the only non-code hit in the tree is an archived doc line); they are absent from the `tracing::info!` at `crates/backtest/src/scenarios/garch_vol_target_overlay.rs:518-524` that *does* render the four sibling counters, from `TcnOverlayRunResult`, and from every test — while `crates/strategy/tests/vol_targeting_overlay.rs:196-200` proves the read is trivially available and uses it for two of the other four. `return_vol_correlation` likewise has zero readers, and ADR-0078's Consequences states it "gives the operator an explicit, per-coin, per-window answer to whether vol-targeting can even mechanistically help" — a stated ADR consequence with no delivery path.

```
# read-only scratch crate, path-dep on crates/strategy, 600 bars per config
A) VolTargetingConfig::default()  band_suppressed=0   derisk_suppressed=0    <- production: pinned at 0
B) p1_4_defaults()                band_suppressed=600 derisk_suppressed=599  <- nothing reads them
```
`VolTargetingConfig` has no `Deserialize` and the two keys appear in no toml/json/yaml, so the policies cannot be enabled from config; `p1_4_defaults()` has three callers, all unit tests in its own file. **Not** an AD-16 no-op overlay: the policies being off is ADR-0078's documented backward-compat decision, and total suppression would flip the anchored body's "Modified by overlay / Modification rate" rows from 100.00% to 0.00%, so the inert outcome *is* rendered.
**Smallest fix:** add the two counters to the existing `tracing::info!` and to `TcnOverlayRunResult` (one line each) or delete them; deliver `return_vol_correlation` to a report annex or amend ADR-0078. Adjacent, not proven here: `on_bar:722-726` tests `|final_scale − 1.0| < tol` while the field's doc at `:448` says "a resize occurred".

### F13 — `Recommendation.benchmark_kpis` / `.winner_kpis` have no reader; both ADR-named consumers re-derive
`crates/backtest/src/bakeoff/mod.rs:901,:903`, written `:1506-1507`.
**Shape:** computed, never consumed.

Zero readers, proven exhaustively rather than by grep alone: one declaration and exactly one construction site (`Recommendation {` appears twice in the tree — `:895`, `:1503`), no destructuring and no `..base`, `#[derive(Debug, Clone)]` only so no serde reader, and no site `{:?}`-formats the struct wholesale. `RecommendationMirror` (`crates/ui/src/leaderboard/state.rs:131-141`) declares only four fields, so there is no field to carry a KPI across the mirror seam; the UI re-locates the benchmark itself at `crates/ui/src/screens/leaderboard.rs:1119-1126` — which *is* the "vs just holding" comparison `benchmark_kpis`' doc comment names as its purpose; and `agent/src/narration.rs:1031-1043` linear-searches `candidate_kpi_strings` for the winner's KPIs rather than reading `winner_kpis`. ADR-0059 §D2 and ADR-0064 both declare the six-field contract live, so the seam is satisfied by re-derivation and two of its fields are an unread parallel copy.

Struck from the original: the all-zero `map_or_else` fallback at `:1403-1416` is genuinely unreachable (`strategy_ids.push((BUYHOLD_ID, true))` at `:1200` is unconditional; the only three `continue`s before the `find` are `is_dvol_arm`/`is_macro_arm` guards; the other early exit returns `Err`), so no wrong value can reach the field today.
**Smallest fix:** delete both fields and the doc sentence, or read them at the two mirror boundaries so the ADR-declared contract is the one in use.

### F14 — Four assertion-free tests certify a byte-identity property for a construction path with no production caller
`crates/exec/tests/paper_engine_publishes.rs:97`; `crates/exec/src/paper.rs:240,:258`.
**Shape:** vacuous assertion + three false sentences.

The body between `#[test]` and the closing brace is a 50-iteration `engine.on_fill(...)` loop and nothing else (`sed -n '93,107p' | grep -c assert` → 0); no recorder is constructed, so the only observable channel is a handle the test never holds, while the sibling at `:80` shows the measurable shape (`Recorder` + `assert_eq!(len, 10)`) was available in the same file. `grep -rn size_of crates/exec/` is empty, so nothing asserts `NullPublisher` is zero-sized or side-effect-free. Drop `publisher.rs:92` from the list — it discloses itself ("Reaching here without panic is the assertion."); keep `paper.rs:258`, which claims `Default` and `new()` are equivalent and compares nothing.

Two load-bearing claims must be struck. (i) Nothing names this test: `grep -rn "t903a" --include=*.md --include=*.toml --include=*.yaml` returns zero hits across every story, trace row, ADR, CHANGELOG line and sprint-status entry. (ii) `scripts/verify_anchors.sh` contains no `cargo` invocation — it hashes committed bodies — so this test was never the mechanism between a `NullPublisher` regression and anchor drift, and the doc comment's "anchor reports would drift" is itself a false sentence. And the guarded path does not exist: `grep -rn PaperEnginePublisher crates/backtest/` → 0; `::new()`/`::default()` appear only at `paper.rs:241,:259`, both inside the exec unit tests; the sole production construction is `with_publisher` at `crates/agent/src/runtime.rs:1818` on the live-bus path. So "Backtests construct `PaperEnginePublisher` with `NullPublisher`" — stated at `publisher.rs:16`, `paper.rs:14` and `runtime.rs:1804` — is false in the tree.
**Smallest fix:** correct the three sentences and the drift claim; optionally add an `AtomicUsize` side-effect counter in the fixture, mirroring the `:80` Recorder.

## 3. Checked and dismissed

| Candidate | Why it is not a finding | Source |
|---|---|---|
| Bear/overfit surveys score the frozen gate with a 3-of-5 `score_verdict` copy (`realdata_simple_strategy_overfit_guard.rs:244`) | Implements a *different* §0 clause, documented "applied AS-IS" in four places; both "dropped" arms are pinned by running, arm-isolating bin-local tests; report is **un-anchored** (stated at its lines 135/148); copy is strictly *more lenient*, so all 26 published rows are identical under both rulers; the p50 arm is unreachable by percentile monotonicity (0 of 16008) | Measurement + `docs/dev-notes/robustness-decision-rule-2026-05-30.md:56`; `param_robustness_sweep.rs` classifier tests |
| `same_paths_seeding_invariant_adr0051_d6_1` tests a local copy of the ADR-0051 D1 seed rule | Found and fixed by the repo's own review: commit `5d26cf10` "seed invariant via the production fn only" added `seeding_invariant_via_production_fn` (`param_sweep_e2e.rs:1077`), and the bin-local `derive_path_seed_matches_adr0051_d1` also pins the formula — both go RED under the claimed sabotage | Probe (both tests fail under the mutation) + `git log -S` |
| `monte_carlo` bin unreachable / MC-P50 verdict ungated | `mc_reproduces_anchor` builds, *runs* and hash-compares the bin; it already caught a real drift (#126, re-emitted under D6.b); only 3 of 4 tests are `#[ignore]`d (the claim's `grep -c` counted a doc comment) | bug-log #126, #93, #113; ADR-0051 §D6.4 |
| `compute_sigma_train` has no test caller | A real gate exists and would fail: `anchored_report_reproduction.rs:509/536` runs the bin and asserts the witness `σ_train = 0.018015675` plus the body SHA. Parked behind `#[ignore]` + `required-features`, which is ADR-0035's declared enforcement | ADR-0035 "Enforced by"; `crates/forecast/Cargo.toml:30-36` |
| `huber_loss` / `onecycle_lr` / `compute_val_loss` / `write_checkpoint` untested | Declared with a stated trade-off ("binaries are anchored through their outputs, not their tests"); checkpoints are committed byte-frozen artifacts, so no edit reaches a shipped weight; programme retired at terminal F4 | `retired-surface-inventory-2026-05-22.md` TL;DR + lesson 2; do-not-build A-2 |
| `theta_surface_reproduction.rs` compiled out of CI; its always-runs companion with it | Disclosed verbatim: "compiled out rather than skipped"; CI non-enablement priced in #121; same shape already filed as #88; `relock_manifest.rs` + `verify_anchors.sh` catch the stated consequences | `anchor-gate-coverage-audit-2026-09-26.md` §6; bug-log #121, #88; ADR-0032:199-201 |
| Three `backtest` test targets silently dropped (118 tests) | Mechanism disclosed (#81 + its 2026-08-15 extension: nothing enables `backtest/{yahoo,realdata}`, no `default` stanza); count is 23 in the targets, ~95 are `#[cfg]`'d code inside targets cargo *does* build; enabling the feature would convert a compile-out into #66's lying guard | bug-log #81, #140; audit §4/§6; ADR-0097 D1/D7 |
| `ui/render-debug`'s six guards compiled by no CI step | Audit row 23, CI ❌, classified "dev/debug opt-in by design", severity "none"; `DebugRenderer` has **zero** non-test construction sites, so it intercepts nothing to guard; `--all-features` builds in rust-validate/rust-coverage do compile and run them | `feature-reachability-audit-2026-08-15.md` row 23; `reachability-map-2026-08-15.md:799` |
| OU positive control executes zero assertions (`null_data_no_crown.rs:720`) | Documented in a dedicated section of the file, recorded in the brief, and **decided** as task T_FINAL_7 "ACCEPTABLE ship state"; the gate is demonstrably not always-conservative (GBM 1/5, GARCH 2/5 crown active arms, DSR assertion executed 3×) | `advisor-no-alpha-gate-ci/tasks.md` T_FINAL_7; `NULL_GATE_DEBUG=1` probe |
| `overlay_hygiene_gate.rs` censuses by filename suffix, missing two overlays | Named verbatim three months ago, with the broader form (sizing modifiers out of scope entirely) and an AD-16 grade of **NO**; same family filed as bug-log #74 | `claims-ledger-2026-08-15.md:306` + §S2 + Appendix "Shape A"; bug-log #74 |
| Coinbase AD-9 `t1403_no_f64_in_money_paths` asserts on `Decimal::from_str`; its nominated grep does not exist | ADR-0003 scopes the rule to audit/exec/risk/strategy (not `data`) and decides the mechanism: "`cargo clippy` rule? Not yet — relies on review + the anchor gate"; the real gate is `Price(Decimal)` + the `core` float lint, so an f64 cannot reach `Tick.price`. Residue is a test **name**, fixable by rename | ADR-0003 Decision + Consequences; `crates/core/src/money.rs:128` |
| Tune screen's `in_sample_sharpe`/`in_sample_maxdd`/`trade_count` reach no column | ADR-0069 §D5 specifies "**the** in-sample point estimate … de-emphasised" (singular) and that one *is* rendered; CHANGELOG:117 states the trade-off; the claim's AD-10 charge is false — `param_sweep_render.rs` is a 9-test `iced_test::screenshot` harness with a negative control | ADR-0069 §D5/§D9; CHANGELOG.md:117 |

## 4. Coverage and its limits

**What the handoff did not carry.** The harness passed me the survived and refuted findings with both verifier verdicts, but **not** the per-angle `what_i_searched`, `cap_hit` or `dropped_note` fields. So the scope below is reconstructed from what each lens's evidence actually touched, and the cap question cannot be answered from the record:

- **`cap_hit` is unknown for every angle, and one angle is at the cap.** The boundary-insensitive lens returned **exactly 4** findings — the brief's per-angle maximum. Either it found exactly four or it truncated; nothing in the handoff says which, and no `dropped_note` accompanied it. Treat that angle's list as possibly incomplete. The other six returned 2, 1, 1, 3, 3 and 0 and are below the cap.

**Swept, by angle.** Boundary-insensitive: numeric threshold constants in `backtest/bakeoff` (`robustness.rs`, `scorecard.rs`), `llm/redact_layer.rs`, `trader/llm_forecaster/verdict.rs`. Re-implements-subject: `crates/forecast/tests/*` (parkinson, recalibrate overlay) plus four candidates that failed (backtest surveys, param-sweep seeding, σ_train, trainers). Unreachable-derivation: `src/bin/*` in backtest/forecast/trader for logic with no test caller. Vacuous assertions: `crates/exec` (paper/publisher) and the forecast `*_readonly` family. Unmeasured counts: `scripts/queue_staleness_check.py`, `operator_ledger_check.py`, `check_presentation.sh`. Computed-never-consumed: `crates/reports`, `crates/strategy/vol_targeting_overlay.rs`, `crates/backtest/bakeoff/mod.rs`. Skip/cannot-compile: `theta_surface_reproduction.rs`, the three `required-features` test targets, `ui/render-debug`, the `monte_carlo` bin.

**Specifically NOT swept.** No threshold census of `crates/risk` (`sizing.rs`), `crates/exec`, `crates/audit` or the UI promotion thresholds — the boundary lens stayed in three crates. No exhaustive diff-based census of test-local helpers against shipped functions: the two re-implementation findings were reached by name-matching, so sibling pairs with renamed copies would be invisible (and `claims-ledger`'s note that sizing modifiers outside `crates/strategy` are out of the overlay census applies here too). No `fn main()` sweep across the 17 crates for inline derivations — only backtest/forecast/trader bins. No workspace-wide census of `#[test]` bodies containing zero assertions. The five sibling scripts already named in `cannot-fail-gate-audit-2026-09-26` (`verify_anchors.sh`, `check_no_clocks_in_ui_tests.sh`, `orch_determinism_check.sh`, `check_no_secrets_in_llm_artifacts.sh`, `precheck.sh`) were left alone as pre-disclosed, as were the count paths inside `spec_lint.py`, `adr_registry_check.py` and `verdict_transcription_check.py` itself. No dead-`pub`-field census in `risk`, `exec`, `audit`, `data` or `ui`.

**Method limits.** No `cargo-mutants` run (its hot tier is strategy/exec/audit/risk and excludes `trader` and `backtest`'s bins anyway). Two findings' "leaves the suite green" rests on a structural argument rather than a mutation, because read-only forbade the edit — F6 (`t_classifier`) and the dismissed `monte_carlo` candidate both say so explicitly. CI behaviour was read from `ci.yml`, `.githooks/*` and `cargo tree -e features`, not from a CI run; `gh` was forbidden, so no check-run annotations were consulted. Every probe ran on copies, and one of them taught the lesson: the frozen-bands reporter's carve-out crate claimed "5 lib + 10 bin tests" where the real crate has **241 + 43 + 84**, so a finding proved only in a stripped `/tmp` crate is weaker than one re-run in-repo. F2, F8 and the F4 `nm` evidence were re-measured against the real crate; F1, F9 and parts of F3 rest on byte-identical carve-outs whose fidelity was checked by `shasum` and by diffing `-- --list` output.

**Prior-art discipline.** All 26 candidates were grepped against `docs/dev-notes/bug-log.md` (#65–#154) and `docs/dev-notes/do-not-build-register.md`; the surviving 14 returned zero hits for their own subjects. Nine of the twelve dismissals were killed by the records rather than by measurement, which is the result worth keeping: the repo's own disclosures are doing real work, and #154 + ADR-0099 — landed today — correctly scoped themselves to three forecast classifiers while leaving the five same-shape instances above outside the gate's table.

## 5. Ranked next actions

1. **Two-sided fixtures for the entropy fallback, and delete the tautological `entropy_high_for_mixed`.** Closes F1: boundary-insensitive fixture + expectation-from-the-measurement, on the AD-19 redaction gate with a demonstrated unredacted-secret consequence. ~30 lines, one file, no anchor impact. **S.**
2. **Four two-sided fixture pairs for the AD-1 ROBUST bands.** Closes F2 for the project's load-bearing invariant; `classify_verdict` stays byte-frozen, only `mod tests` grows. Each pair must hold the other four criteria clear by construction and first assert its fixture lands where intended; percentile monotonicity (p5 ≤ p50 through `DistributionSummary::from_path_metrics`) constrains what is constructible, so budget for that. No anchor impact. **M.**
3. **Make the two phantom gates call their subjects.** Closes F3 and F4: one in-crate `#[cfg(test)]` test in `features.rs` driving the real Parkinson arm via `WindowIterator::new` (no parquet needed), and move `write_overlay` into `forecast::provenance` so the existing field-invariance test asserts against the shipped function. Also correct the three artifacts citing `parkinson_target_derivation` as the gate. **M.** ⚠ The second half, if the report's hardcoded `✓` invariance table is also made a real comparison, **moves the two `recalibrate-sigma-train-bs{1,2}` anchored body-SHAs** — operator ruling required under ADR-0038 §D6.b before touching `evidence/`.
4. **Non-vacuity floors and parsed-row counts on the three `scripts/` gates.** Closes F5, F7, F10: a regex that reads the committed story-id markup plus "N of M reconciled" and exit-non-zero-at-zero in `queue_staleness_check.py`; a required-table assertion raising the already-documented exit 2 plus a row count in `operator_ledger_check.py`; an approval-box count with exit 2 at zero in `check_presentation.sh`. Update `queue_staleness_check.py`'s self-test fixtures to post-migration markup in the same pass, since they are why the regression was invisible. No anchor impact. **S–M.**
5. **Boundary fixtures for the three published verdicts.** Closes F6 (both `t_classifier` edges, in the `#[cfg(test)]` module that already exists), F8 (one computed true-side `crown_clears_dsr` assertion plus make the three worked examples read the constant) and F9 (adjacent pairs for L1, L3, L4). Tests only — do **not** move any threshold and do **not** wire DSR as a crown veto (do-not-build E-1; ADR-0075 D2). Where the pairs fit ADR-0099's model, add them to `verdict_transcription_check.py`'s table so the next instance is measured rather than audited. No anchor impact. **M.**

**Deliberately below the line, not dropped:** F11–F14 (`body_sha256`, the vol-overlay counters and `return_vol_correlation`, the two `Recommendation` KPI fields, and the `paper_engine_publishes` family). All four are dead-value or stale-sentence hygiene where no gate loses teeth and no published number can be wrong today — but each carries a false doc comment or ADR consequence that a future reader would act on, and F11 additionally hides a one-byte disagreement with `hash_report.py` that would bite the first consumer to trust the field. One combined cleanup pass, **S**, no anchor impact.

## The completeness critic, verbatim

# Completeness critique — cannot-fail-gate sweep, 2026-11-07

`cap_hit: true` · `already_disclosed: none of F-A..F-D` (greps below)

The sweep's own section 4 is honest about *which files* it skipped. What it does not say is that one of the areas it listed as skipped contains a finding of the strongest class, reachable on the first look — so "specifically NOT swept" is not a priced residual, it is the sweep's largest defect. Four findings, strongest first; three dropped findings are folded into Q1/Q2 where they are directly actionable.

---

## F-A — 2 of the 4 AD-9 exact-cent reconciliation identities compare an expression to itself, and no angle searched the shape

`crates/reports/src/lib.rs:182,185-187,190-191` (production wiring) · pairing at `crates/reports/src/reconcile.rs:102-106,119-122` · `passed = (delta == Decimal::ZERO)` at `:51`.

**Shape:** the expectation and the measurement come from the same place — the repo's *first-listed* known sub-shape, which has **no angle** in the seven. "reimplements-subject" looks at a test holding a copy of its subject; it does not ask whether the two sides of a shipped identity are independently sourced.

```
$ sed -n '190,191p' crates/reports/src/lib.rs | sed 's/^ *[a-z_]*: //' | sort -u
realized_period.amount() + unrealized,
(distinct expressions: 1)
```

Identity 4 (`equity_delta = realized + unrealized + fees_delta`) is the **same expression typed twice** into the two struct fields that `compute` subtracts, so `delta == ZERO` unconditionally, for every input. Identity 1 is the same defect one hop out: `lib.rs:182` *defines* `headline_return_usdt = realized.amount() + unrealized`, `:185-187` pass that and its two summands separately, and `reconcile.rs:102-106` pairs `inputs.headline_return` against `inputs.realized + inputs.unrealized`. Only identities 2 and 3 (`sum_by_strategy` / `sum_by_symbol` vs the inception-side `realized`) are independently sourced — and those are exactly the two the only FAIL test breaks (`crates/reports/tests/reconciliation_mismatch.rs:4-11`).

The label also names a term that is computed nowhere:

```
$ grep -rn "fees_delta" --include='*.rs' crates/ | grep -v '//'
crates/reports/src/reconcile.rs:119:        "equity_delta = realized + unrealized + fees_delta",
```

Shipped consequence — four committed operator-facing bodies publish four PASS rows for the double-entry gate:

```
$ grep -A6 "^## Reconciliation" evidence/v1/paper-soak-longevity/reports/operator-success-soak-2026-06-19.md
| headline_return = realized + unrealized            | 0 | 0 | 0 | PASS |   <- tautology
| Σ pnl_by_strategy = Σ realized                     | 0 | 0 | 0 | PASS |
| Σ pnl_by_symbol = Σ realized                       | 0 | 0 | 0 | PASS |
| equity_delta = realized + unrealized + fees_delta  | 0 | 0 | 0 | PASS |   <- tautology
```
(same table in `success-fixed-report-sample-{7d,90d}.md` and `operator-success-research-replay-2026-06-19.md`.)

Why no existing test sees it: every fixture builds `ReconciliationInputs` by hand (`tests/reconciliation.rs:14`, `src/render/reconciliation.rs:52`, `src/reconcile.rs:224`, all `dec!(140.00)`), so they test `compute`'s arithmetic, never the wiring. `grep -rn equity_check_sum --include='*.rs' crates/` returns exactly one production assignment (`lib.rs:191`) and three fixtures.

`already_disclosed: no.` `grep -in "reconcil|equity_check_sum|fees_delta|exact-cent" docs/dev-notes/bug-log.md docs/dev-notes/do-not-build-register.md` returns only #99/#131 (unrelated tape/anchor reconcilers). `grep -rn "fees_delta|equity_check_sum" --include='*.md' --include='*.toml' --include='*.yaml' .` returns **only the four shipped bodies** — no ADR, story, trace row or dev-note declares identity 4 a deliberate placeholder. Partial disclosure exists for identity 1 only, and framed as a property of one fixture rather than of the wiring: `reconciliation_mismatch.rs:7-8`, "The headline identity still balances (both sides see the same `realized + 0`)".

---

## F-B — the repo's own "never a re-implementation" rule is written in one test and broken in the AD-2 anchor test; the gate landed today covers 3 pairs out of ~81 candidates

The reimplements-subject angle sampled **4 files**. One command censuses the shape workspace-wide:

```
$ for c in crates/*/; do comm -12 <(grep -rhoE "^ *(pub )?fn [a-z_0-9]+" $c/src --include='*.rs' | sed -E 's/.*fn //' | sort -u) \
                                 <(grep -rhoE "^ *fn [a-z_0-9]+"        $c/tests --include='*.rs' | sed -E 's/.*fn //' | sort -u); done
```
→ **81 shared src/tests function names in 13 of the 14 crates that have a `tests/` dir** (ui 17, agent 9, forecast 9, llm 9, backtest 7, strategy 6, trader 6, exec 5, reports 5, core 4, audit 2, data 1, reflection 1; cost 0, risk 0). A name match is a candidate, not a copy — but the one I followed is a confirmed hit on AD-2:

```
$ grep -rn "fn strip_front_matter" crates/ scripts/
crates/reports/src/parse.rs:123                        (find_closing_fence, handles CRLF + empty front matter)
crates/reports/tests/strategy_anchors_unchanged.rs:495 (strip_prefix + find("\n---\n"), map_or(""))
crates/ui/src/reports/loader.rs:256                    (find("\n---"), no trailing-newline requirement)
# plus scripts/hash_report.py:20  FRONTMATTER_RE = ^---\n.*?\n---\n   <- the canonical one
$ grep -n "^use " crates/reports/tests/strategy_anchors_unchanged.rs
75:use sha2::{Digest, Sha256};
76:use std::path::{Path, PathBuf};
```

So the file that asserts **9 AD-2 anchor SHAs** (`grep -c assert` → 8) lives in `crates/reports/tests/` and contains **no code from the `reports` crate and no call to the canonical hasher** — it re-derives both the strip rule and `body_sha256` locally (`:488-502`), with a doc comment asserting it matches the python regex. Four independent implementations of the body-hash boundary exist and nothing compares any two of them. The house rule is already written down, in a sibling: `crates/forecast/tests/anchored_report_reproduction.rs:846-849` — "The repo's OWN hasher, never a re-implementation — scripts/hash_report.py is what…" — and shells out. Three forecast tests do it that way; this one does not.

And the remedy landed this morning does not reach it: `python3 -c` on the script shows `BRANCH_PAIRS` has **3** entries and the input-fidelity check **1**, all three forecast verdict classifiers, hand-written with no census step — so a 4th transcription pair is invisible by construction. The sweep's own F3/F4 found two more pairs in that same crate; the census above offers ~79 more leads.

`already_disclosed: no` for the strip_front_matter fan-out (`grep -n "strip_front_matter" docs/dev-notes/bug-log.md` → 0 hits; #107-adjacent entries at `:2328,:4113` are a different re-implementation, of `run_scenario_once` and of a sharpe renderer).

---

## F-C — 8 of 17 crates received zero candidates, including both crates that carry AD-9

```
$ # candidate crates: backtest llm trader forecast ui exec data strategy reports  (9 of 17)
$ wc -l crates/audit/src/*.rs crates/core/src/money.rs | tail -1
   7824 total        # audit 7,610 + core/money.rs 214
$ ls crates/audit/tests | wc -l   →  39
$ ls crates/risk/tests            →  budget_sizing_divergence_end_to_end.rs   (1 target, whole crate)
```
Never swept: **agent, audit, core, cost, features, reflection, replay-cache, risk**. That set contains the double-entry ledger (`audit/src/{journal,query,ledger,equity_store}.rs`), the `Money<C>` newtype and the Decimal rule they enforce, and the position sizer. AD-9 and the exact-cent ledger are named in the brief's own consequence weighting; the sweep's 26 candidates reach neither.

The measure of the gap is F-A: the first money-math file opened produced a tautological gate on a non-negotiable. By contrast, the exec/fill path got one candidate (`paper_engine_publishes.rs`) which the report itself ranks last and files "below the line", and the UI — AD-10, a non-negotiable with 135 src files and 114 test targets — got **2** candidate files (`widgets/debug_renderer.rs`, `tune/state.rs`), neither of them a screenshot harness or a visual baseline.

---

## F-D — the three gates that are actually enforced were not swept; the three scripts that were swept are enforced nowhere

```
$ grep -rn "queue_staleness\|operator_ledger\|check_presentation" .github/ .githooks/
(no output)
$ sed -n '80,89p' .github/workflows/ci.yml   # + .githooks/pre-commit, identical set
bash scripts/verify_anchors.sh | tail -1
python3 scripts/spec_lint.py ; python3 scripts/spec_lint.py --self-test
python3 scripts/verdict_transcription_check.py ; python3 scripts/verdict_transcription_check.py --self-test
$ wc -l scripts/spec_lint.py scripts/verify_anchors.sh scripts/verdict_transcription_check.py
1091  294  440
```
The unmeasured-counts angle swept three advisory scripts and zero enforced ones. All three findings it produced (F5, F7, F10) are therefore about tools no commit and no CI run ever invokes, while the 1,825 lines that gate every commit went unexamined — including `spec_lint.py`, which is the sole mechanical enforcement of the ADR-0082 triad and has been audited by neither sweep:

```
$ for s in verify_anchors spec_lint verdict_transcription hash_report; do \
    echo -n "$s: "; grep -c $s docs/dev-notes/cannot-fail-gate-audit-2026-09-26.md; done
verify_anchors: 3    spec_lint: 0    verdict_transcription: 0    hash_report: 0
```
The report's claim that these were "left alone as pre-disclosed" is true only of `verify_anchors.sh`. Note the precedent sitting in the same CI step: bug-log #84 — `verify_anchors.sh | tail -1` under a non-pipefail shell meant AD-2's only remote enforcement "could not fail" from CI activation onward. That is this exact shape, in this exact file, and it is why the enforced chain is the wrong thing to skip.

---

`dropped_note:` **3 dropped.** (1) The #150 partly-live-stamp angle: 31 src files across 11 crates call a live clock, including the anchored-body emitters, while the only clock gate hardcodes 8 `crates/ui/` files. (2) The runtime (as opposed to `cfg`) skip-as-pass census: 25 test files early-return on a `exists()` miss, 61 mention SKIP. (3) The #149/#141 rendered-text-integrity angle: 3 of 11 `reports/src/render/*.rs` modules emit a pipe table and nothing asserts one renders as a table. All three are the substance of Q1 below, so they are stated there as angles rather than counted twice as findings.

---

# 1. Which sub-shape was NOT searched by any of the seven angles

Four, concretely. The sweep was adequate on skip-as-pass-by-`cfg` and on computed-never-consumed-at-the-struct-field; it never looked for these.

**(a) Same-source expectation — the house list's *first* shape — has no angle, and none of the seven is a substitute.** "reimplements-subject" asks whether a *test* holds a copy; nobody asked whether the two sides of a *shipped* comparison are independently derived. Proven instance: F-A, `crates/reports/src/lib.rs:190-191` (one expression, two fields, `sed | sort -u` → 1 line). Next sweep's angle, stated mechanically: for every struct whose consumer subtracts or equates two of its fields, diff the two construction expressions. Where the remaining ones live: `crates/audit/src/query.rs` (3,548 lines — both sides of every identity above originate there), `crates/reports/src/lib.rs:180-191`, and `crates/backtest/src/bakeoff/` identity tests. The sweep's F1 brushed this shape (`assert!(shannon_entropy(FIXTURE) >= ENTROPY_THRESHOLD)`) and labelled it a stack on top of boundary-insensitivity rather than the primary.

**(b) The #150 partly-live/partly-frozen stamp has no angle, and the one gate is scoped to 8 files.**
```
$ grep -rl "Utc::now()\|Local::now()\|SystemTime::now()" --include='*.rs' crates/*/src | cut -d/ -f2 | sort | uniq -c | sort -rn
6 ui   6 strategy   6 forecast   3 backtest   2 trader   2 core   2 agent   1 reports   1 llm   1 exec   1 data
$ sed -n '34,41p' scripts/check_no_clocks_in_ui_tests.sh    # the gate's entire corpus
crates/ui/src/widgets/chart.rs … crates/ui/tests/fixtures/visual_diff.rs   (8 hardcoded paths)
```
31 files, 11 crates, one gate covering 8 of them — and the uncovered set includes every anchored-body emitter. Paths to start: `crates/backtest/src/bin/param_robustness_sweep.rs:2252,2347`, `crates/forecast/src/bin/{vol_verdict.rs:618,636, regime_verdict.rs:910, sharpe_comparison.rs:1965, recalibrate_sigma_train.rs:662,674}`, and `crates/reports/src/render/front_matter.rs`. The question per site is the one #150 answers wrongly: which bytes of the stamp are live, which are literal, and does the body-vs-front-matter boundary actually hold.

**(c) #149 (a table that does not render) and #141 (a value that dies at the aggregation boundary) have no angle — computed-never-consumed stops one layer too early.** It asks "does anything read this field"; it never asks "does the rendered row render, and does its label name what was computed". F-A's `fees_delta` is the proof that this layer is productive: the label is in no field and no computation, yet it ships in four operator-facing bodies. Paths: the three table emitters `crates/reports/src/render/{reconciliation,risk_metrics,strategy_attribution}.rs` and `to_appendix_table` at `crates/reports/src/reconcile.rs:159`; the cheap mechanical form of the angle is to parse every committed `evidence/**/reports/*.md` for pipe-table well-formedness and for identity labels whose named terms do not appear in the producing struct.

**(d) Runtime corpus-gated skip, as distinct from `cfg`.** The feature-gated-skips angle swept three compile-time artifacts. `grep -rn "exists() *{" crates/*/tests/` → 25 files; `grep -rln "SKIP\|skipping" crates/*/tests/` → 61. CI even has an "informational" step for two of them, which prices two and leaves the rest. Paths: `crates/backtest/tests/binance_cache_dispatch.rs:572,589`, `crates/forecast/tests/smoke_train.rs:298,305,328,336`, `crates/backtest/tests/yahoo_report_helper_shape.rs:157,210`. (Caution, and the reason this is an angle rather than a finding: I nearly filed `crates/trader/tests/llm_forecaster_neutrality.rs` as a live self-skipping AD-2 neutrality gate on a `grep | head` that truncated before line 49 — it *is* `#[ignore]`d, with the reason given. The angle must read each attribute, not grep for it.)

A fifth, smaller: **the self-test as the thing that cannot fail.** The sweep proved one instance (F5's fixtures frozen at pre-migration markup) and treated it as a detail of that finding. Four `--self-test` entry points exist and two of them are the CI gate (`spec_lint.py`, `verdict_transcription_check.py`); each builds its own fixtures from hardcoded canonical forms, which is precisely the mechanism F5 and F7 died of.

# 2. Which high-consequence subsystem got little or no attention

**Money math and the double-entry ledger — and the answer is not "little", it is none.** Evidence and consequence are F-C and F-A: zero of 26 candidates in `crates/audit` (7,610 src lines, 39 test targets), `crates/core/money.rs` (the `Money<C>` newtype AD-9 names), `crates/risk` (position sizing, one test target for the whole crate) or `crates/reports`'s reconciliation module; and the first file opened there published a double-entry gate whose delta is structurally zero, in four operator-facing bodies, with no ADR, story or bug-log entry anywhere in the tree. The brief's own weighting puts "money math (Decimal, the double-entry ledger)" first; the sweep's coverage section lists `crates/risk`/`crates/exec`/`crates/audit` as un-swept and then treats that as priced. It is not priced — it is where the finding was.

Runner-up, by the same arithmetic: the **UI rendered-pixel harnesses** (AD-10). 135 src files + 114 test targets, 2 candidates, neither a screenshot harness nor a visual baseline — and the 56 visual baselines are regenerated from the code they gate, which is shape (a) in its purest form and has no angle. `crates/ui` also carries 17 of the 81 shared src/tests names in F-B's census, including `render`, `band_label` and `tape_summary`.

Adequate, for the record, and I will not invent work there: the FROZEN robustness gate's coverage (F2 re-measured 368 shipped tests in-repo with two controls) and anchored report *hashing* as a corpus (F11 plus bug-log #84/#131/#142 cover the gate's own mechanics) were genuinely well served.

# 3. Weakest claim, and the one command that settles it

**F11's load-bearing reassurance: "AD-2 is not weakened and must not be claimed: `crates/backtest` has no `reports` dependency, so `reports::generate` writes none of the 119 anchored files."** It is the single sentence in the report that tells a reader to stop worrying about a hash disagreement in the anchor corpus, and its stated premise is false:

```
$ grep -n "reports" crates/backtest/Cargo.toml
128:[dev-dependencies]
140:reports = { path = "../reports" }
```

The command that settles it — and the one the sweep needed before writing that sentence:

```
$ cargo tree -p backtest -e normal -i reports
warning: nothing to print.
$ cargo tree -p backtest -e normal,dev -i reports
reports v0.1.0 (…/crates/reports)
[dev-dependencies]
└── backtest v0.1.0 (…/crates/backtest)
```

The conclusion survives on the **normal**-edge reading, which is the reading that matters for `src/` and the bins — so F11 is right by luck, not by proof. The dev edge is live for `crates/backtest/tests/*`, which is exactly where anchored bodies are re-emitted and compared, and the report never makes the normal/dev distinction its claim depends on. One `grep` away, in the sweep's own strongest-evidence crate, on the claim that closes the question — the same failure mode the brief opens with.

Honourable mention, since the report volunteers it and a critic should not award the prize for candour: F6 says in its own text that "leaves the suite green" is a structural argument and not a mutation. That is a disclosed limit. F11's is an undisclosed one.

## Provenance

Run `wf_d150bf28-c06`. 61 agents, 0 errors, 1 946 tool calls, 7.87 M subagent tokens,
78 min wall-clock. Per-agent returns: the run journal under the session subagents directory.
One verification agent’s structured output tripped the harness’ `marker-prefix-forgery` detector;
its control tags were neutralised in place and nothing in it was treated as an instruction.
