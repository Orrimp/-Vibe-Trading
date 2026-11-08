# The 2026-11-08 re-emission pass — ADR-0038 § D6.b + the new § D6.c

**Scope:** every anchored body rendered by a writer under `crates/backtest/src/report/`.
**Protocols:** [ADR-0038 § D6.b](../../_bmad-output/planning-artifacts/architecture/decisions/0038-vol-forecast-verdict-shape.md)
(figures moved) and **§ D6.c** (content added), authored in this pass because D6.b does not
cover an additive content change — see its Changelog entry.
**Bug-logs:** [`#95`](bug-log.md) · [`#151`](bug-log.md) · [`#176`](bug-log.md) ·
[`#177`](bug-log.md) · [`#178`](bug-log.md).

## Why one pass and not three

Three changes landed against the same renderers on the same day, and the corpus is one
corpus:

1. **`#95`** — five lanes declared `portfolio_exposure_cap: Some(0.50)` and nothing read it.
   Now enforced by a per-order entry gate on RESULTING exposure. On two lanes this **moves
   figures** → § D6.b rows.
2. **`#151`** — no lane body carried a benchmark, so every return it printed was unjudgeable.
   Now every lane table carries an equal-weight buy-and-hold control and a delta in
   percentage points. This **adds content without moving figures** → § D6.c rows.
3. **`#178`** — the body's risk sentence was a literal, and `#95` made it false within the
   hour. Now derived from the limits the lane ran with. Content, not figures → § D6.c.

Re-emitting these separately would be three passes over one corpus for one result, and
`#177` shows what a partial pass costs: the 2026-10-29 re-lock covered the canonical
namespace only, and ten reproduction gates sat red for three weeks.

## § D6.b step 2 — the smoking gun, with `file:line`

At `f54cab2a` (the commit before the gate landed), each lane declared the cap and no code
read it:

| lane | declaration |
|------|-------------|
| `scenarios/patchtst_overlay_weights.rs` | `:155  portfolio_exposure_cap: Some(dec!(0.50)),` |
| `scenarios/tcn_overlay_weights.rs` | `:148  portfolio_exposure_cap: Some(dec!(0.50)),` |
| `scenarios/regime_dispatcher.rs` | `:246  portfolio_exposure_cap: Some(dec!(0.50)),` |
| `scenarios/threshold_sweep.rs` | `:156  portfolio_exposure_cap: Some(dec!(0.50)),` |

The only enforcer in the tree is `risk::size_portfolio_target`, which needs a whole target
VECTOR; its only production caller is `scenarios/montecarlo.rs:534`, and none of the four
lanes above builds a vector — each emits one order at a time. So the field was present in
the struct, printed in the body, and consulted nowhere. That is the same shape as D6.b's
founding case: a control that was computed and never applied.

## § D6.b step 3 — the would-have-caught test, run against the PRE-FIX code

`crates/backtest/tests/gross_cap_declared_is_gross_cap_enforced.rs`. Not a one-off: the test
honours `GROSS_CAP_SCAN_DIR`, so the pre-fix FAIL is re-derivable by anyone, forever.

```bash
mkdir -p /tmp/prefix && git archive f54cab2a crates/backtest/src/scenarios \
  | tar -x -C /tmp/prefix --strip-components=4
GROSS_CAP_SCAN_DIR=/tmp/prefix cargo test -p backtest \
  --test gross_cap_declared_is_gross_cap_enforced
```

Literal output, captured 2026-11-08:

```
these scenario lanes declare `portfolio_exposure_cap: Some(..)` and enforce it with
NEITHER mechanism: ["patchtst_overlay_weights.rs", "regime_dispatcher.rs",
"tcn_overlay_weights.rs", "threshold_sweep.rs"]
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

**The gate names four lanes.** My own `#95` census said three — I had filed
`threshold_sweep` with the θ-SURFACE anchors, which are `montecarlo`'s. The gate corrected
the enumeration it was written to enforce, which is the only reason step 3 asks for the
pre-fix run rather than a description of it.

Unset (how CI runs it) the same test passes 2/2 on this checkout; an empty value falls back;
a wrong path fails loudly rather than scanning nothing and reporting compliance.

## § D6.c step 2 — the enumeration, mechanically

`scripts/reemission_classify.py --new-dir <dir>`. It imports `body_hash` from
`scripts/hash_report.py` rather than reimplementing it, so this tool and
`verify_anchors.sh` cannot drift on the one definition they share (ADR-0097 § D3's rule). It
has a `--self-test` with three probes and a control.

Each anchor row lands in exactly one bucket:

- `UNCHANGED` — the new body hashes to the row's SHA. The § D6.c step 6 negative invariant.
- `D6C` — differs, and every differing line is in the declared additive set:
  `| Buy and hold (equal-wt)`, `| vs buy and hold`, `- Risk:`, `- Size:`.
- `D6B` — differs in a FIGURE line (`Final equity`, `Total return`, `Max drawdown`,
  `Trades`, `Buys`, `Sells`, `Total fees`, `Gross-cap refusals`, `Sharpe`,
  `Initial capital`) or in any undeclared line. Refused by § D6.c step 4.
- `UNRUN` — no new body, because the scenario is rendered by code outside the `backtest`
  CLI. Verified mechanically: the changed writers are called **only** from
  `backtest/src/main.rs`, `backtest/src/engine.rs` and `backtest/src/bin/run_yahoo_sma.rs`.
  `crates/forecast`'s sharpe-comparison and verdict bins, `crates/reports`, and the
  threshold-sweep bin do not touch them, so those rows are unchanged by construction. Said
  out loud rather than counted as compliance.

**Consequence worth stating plainly:** `regime-verdict-bs1-realdata` and
`vol-verdict-bs1-realdata` are rendered by the verdict bins, not by
`report/regime_dispatcher.rs`. So **`#151` moves no verdict body.** A verdict can still move
only through `#95` changing a lane's figures, which is why the refusal count per lane is
recorded below rather than assumed.

## How the bodies were produced

`target/release/backtest --scenario <name> --seed 0xC0FFEE --reports-dir <scratch>` for each
of the 77 distinct anchored scenarios. Two details that are not optional:

- **Release, not debug.** The R-REPRO gates use the release binary; a debug run of the
  regime lane took over 30 minutes and was abandoned.
- **`--reports-dir` OUTSIDE `evidence/`.** A fresh body written inside the corpus can win
  `verify_anchors.sh`'s `find | sort | tail -1` resolution and flip the gate as a side
  effect — bug-log `#113`, which is why the flag exists.

All 80 committed bodies that print a seed print `0xC0FFEE`, so the seed is not a confound.

## Classification result

*(Appended when the run completes — see § Result below.)*

## The invocation matrix — which command produces which namespace

This is the piece that was not written down in one place, and the reason a re-emission
looks impossible until you find it. **119 rows over 77 scenarios: 41 scenarios carry one
row, 15 carry several rows that share one SHA, and 21 carry rows with DIFFERING SHAs** —
those 21 need more than one invocation, because each row names a different friction
condition.

I first concluded the conditions were unrecorded. That was wrong, and wrong the same way as
three other claims in this day's bug-log entries: I had not looked in the right place. They
are declared in two places, and between them they determine every row.

**1. `evidence/anchors.toml` comments** — re-emitted rows carry a `Condition DECLARED`
line, e.g.

```
#   Condition DECLARED (bug-log #123): default real-data (Linear{bps:0}, zero sim slippage).
#   Condition DECLARED (bug-log #123): default synthetic (Linear{bps:8}, forced).
#   Condition DECLARED (bug-log #123): DECLARED sqrt flags: alpha=1.0 lookback=90 (ruling (a)).
```

**2. The Q-D1=(a) decision in `crates/backtest/src/main.rs`** (`7e8a7e03`, 2026-05-29),
stated at `:33`, `:164`, `:184`, `:197`, `:214`: a **synthetic** scenario uses
`SlippageModel::Linear { bps: 8 }` **regardless of CLI flags**; only the scenarios listed in
`REAL_DATA_SCENARIO_IDS` honour them.

Putting those together:

| scenario kind | invocation | namespace the body belongs to |
|---|---|---|
| synthetic (anything not in `REAL_DATA_SCENARIO_IDS`) | default — flags are ignored here | **canonical** `+ v5-realdata-medium-2026-05` |
| real-data | default | **`+ noop-baseline`** (zero sim slippage) |
| real-data | `--sim-slippage-bps 8` | **canonical** `+ v5-realdata-medium-2026-05` |
| real-data | `--sim-slippage-sqrt-alpha 1.0 --sim-slippage-sqrt-lookback-days 90` | **`v5-sqrt-impact-2026-05`** |

The nine scenarios honouring the CLI — and therefore the only ones needing more than one
run — are `top10-2023-fy-momentum-realdata`, `top10-{2023,2024}-fy-tcn-overlay-realdata`,
`top10-{2023,2024}-fy-tcn-overlay-weights-realdata`,
`top10-2023-fy-patchtst-overlay-realdata`,
`top10-{2023,2024}-fy-regime-dispatcher-realdata`, and
`top10-2023-fy-vol-target-overlay-realdata`.

**This explains the one observation that did not fit.** The synthetic
`top10-2023-fy-tcn-overlay-weights` reproduced `a25a3693…` byte-identically on a default
run — and `a25a3693…` is its **canonical** row, not its noop row (`7cb1357c…`). Not a
contradiction: Q-D1=(a) forces `Linear{bps:8}` on that scenario, so a default run of a
synthetic scenario IS the canonical condition. The same default run of a *real-data*
scenario is the noop condition. One command, two namespaces, depending on the scenario — and
nothing in the body says which, because the body prints the ORDER slippage
(`Slippage: 2 bps`) and never the SIM-slippage model that distinguishes the namespaces.

**That last sentence is the finding under the finding.** An anchored body does not record
the condition it was produced under. The corpus's own house rule — declare the condition,
make the success path observable — is not met by the corpus. It is why `#177`'s ten gates
could sit red for three weeks: to check them you must first reconstruct an invocation from
an ADR decision, a comment block, and a scenario-ID list in `main.rs`. Writing that
reconstruction down here is the cheap half of the fix; printing the sim-slippage model into
the body (and so into the hash) is the real one, and it is itself a re-emission.

## Result — and the two invocations the first classification pass got wrong

The classifier earned its existence twice before producing a usable answer. Both corrections
came from it refusing to accept a row, not from me noticing.

**A fourth invocation: `--force-synthetic-bars`.** The first run put
`btc-2023-1m-sma-cross` and `btc-2023-1m-sma-baseline-refresh` in D6B with **all eight**
figures moved — Trades 12077 → 441, Total return −82.01% → +7.38%, Sharpe −28.93 → +7.80 —
on SMA lanes that declare no portfolio cap and contain no gate call. Nothing in this diff can
touch them. Reading each committed body's own `data_source:` explained it: those rows pin a
**synthetic** body at 525,601 one-minute bars, and a default run auto-detects the real
parquet data and produces 17,544 hourly bars instead. The override is
`--force-synthetic-bars` (ADR-0047 § D1, the v0.3.0 Q1=(a) guard). Re-running with it moved
both rows to **D6C, content-only** — which is the proof the condition, not the code, was
wrong.

The corpus is not uniform here and the row is what counts: `btc-2023-1m-macd-trend`'s
canonical row on the *same* data is a REAL body at 17,544 bars, which the default run
reproduces. Two sibling scenarios, two different pinned conditions.

**A fifth bucket: `ORACLE`.** A `noop-baseline` row on a SYNTHETIC scenario is a frozen
pre-friction oracle — Q-D1=(a) forces `Linear{bps:8}` on those scenarios *regardless of
flags*, so the zero-slippage condition the row pins is unreachable from current code **by
design**. `anchors.toml` says so itself: "pre-friction historical oracle, SHAs from v0.1.0
ship". The first pass had **28 of these in D6B with every figure moved** — true, and
completely the wrong conclusion. Re-emitting one would not repair a stale pin; it would
destroy the pre-friction baseline the corpus keeps deliberately. They are now excluded and
counted.

**The heuristic both corrections share is now in the tool.** When *every* shared figure
moves, suspect the invocation, not the code: a code change moves some numbers, a different
data source or friction model moves all of them. The classifier prints
`!! ALL n shared figures moved — suspect a WRONG INVOCATION, not drift`. After both
corrections, **zero rows carry that warning** — which is the signal that the remaining D6B
rows are real drift.

## The D6B rows, quantified rather than inferred

§ D6.b's own founding comment sets the standard — "QUANTIFIED, not inferred" — so each row's
cause must account for its delta arithmetically.

**The `#95` rows.** Every refused order is exactly one buy that never happened and one later
sell that never happened:

| row | refusals | Buys | Trades |
|---|---|---|---|
| `top10-2024-fy-tcn-overlay-weights` | 3 | 1995 → 1992 (**−3**) | 3987 → 3981 (**−6 = 3+3**) |
| `top10-2023-fy-regime-dispatcher-realdata` | 20 | 3425 → 3405 (**−20**) | 6847 → 6807 (**−40 = 20+20**) |

Measured refusal counts across the lanes: patchtst-2023 **294**, regime-2023 **20**,
regime-2024 **10**, tcn-weights-2023-realdata **9**, tcn-weights-2024-realdata **12**,
tcn-weights-2024 synthetic **3**, vol-target-2023 **1469** (garch, unchanged — it was already
enforced, and reproducing 1469 exactly is the proof that lifting its two helpers into
`risk::` was behaviour-preserving).

**The six `noop-baseline` rows decompose into two causes, both exact.** For
`top10-2023-fy-tcn-overlay-weights-realdata`:

| body | Trades | why |
|---|---|---|
| committed `noop-baseline` row | 1138 | pre-`#111`: cross-symbol orders vanished with no fill |
| committed canonical row (swept 2026-10-29) | 6203 | post-`#111` fill routing |
| my fresh default run | **6185** | 6203 **− 18** = 6203 − 2×9 refusals |

`6203 − 6185 = 18 = 2 × 9`. So the noop row's staleness is October's `#111/#129` fill-routing
fix — a correctness fix swept for the canonical namespace and not for this one — and my gate
accounts for exactly the remainder. Neither number is an estimate. That the friction flag
changes prices but not the decision sequence is the same property the `eth-2024-h1-sma-cross`
re-emission relied on: "Trades/Buys/Sells are UNCHANGED, so this is a pricing change, not a
decision change."

## REFUTED — I inferred the namespace mapping from reproduction, which this project's own record forbids

I concluded, from three scenarios whose default-invocation body hashed to their **canonical**
row, that the nine R-REPRO gates were pinned to the wrong row and the fix was to re-point
their constants. I put that to an adversarial check before acting on it. It was **refuted**,
and the two citations that settle it verify verbatim:

**1. Frozen evidence defines the namespaces, and canonical is 8 bps.**
`evidence/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/sharpe-delta-2026-05-29.md:14-17`:

> "Linear baseline = v0.4.0 re-emission under v5-realdata-medium-2026-05 namespace (**8 bps
> linear slippage**). Sqrt = v0.5.0 run under v5-sqrt-impact-2026-05 namespace (alpha=1.0,
> 90-day Binance parquet V proxy). **Noop baseline = pre-v5 zero-sim-slippage report**"

`anchors.toml:9-11` says the same: noop-baseline is the "pre-friction historical oracle",
`v5-realdata-medium-2026-05` the "canonical friction".

**2. `#123` forbids my method by name.** `docs/dev-notes/bug-log.md:2981-2983`:

> "For this whole family **no constructible invocation reproduces any anchored row**, which
> means the row assignment cannot be settled by reproduction — only by provenance. … its own
> suggested upgrade test presupposes an undrifted row, which is the thing that fails."

Since every row in the family is drifted, a body matching one row is not evidence it belongs
to that row. My three "exact matches" are explained the other way: the 2026-10-29 re-lock
wrote **default (zero-slippage)** output into the **canonical** rows, so of course a default
run now reproduces them. My own `--sim-slippage-bps 8` measurement is the corroboration —
`top10-2023-fy-tcn-overlay-realdata` at **−55.88%** against the canonical row's **−27.52%**,
a 28 pp gap that is exactly what a canonical row holding zero-slippage content looks like.

The source I had leaned on, `relock-lane-deltas-2026-10-29.md:105-108`, asserts that canonical
"is the namespace a default run reproduces" with no derivation and no code citation, against
both the frozen evidence and `#123`. Its own table betrays the conflation: for
`top10-2023-fy-tcn-overlay-realdata` it reports "−89.89% → −27.52%", and −89.89% is the
**sqrt** row's body, not a canonical one.

### So the state of the corpus is worse than `#177` said, in a specific way

| rows | state | what they need |
|---|---|---|
| 9 `noop-baseline` realdata rows | **stale** — hold the `#67`-broken state (1138 trades) that `#111/#129` fixed (→6203) | re-emit with the **default** invocation |
| 6 canonical realdata rows | **corrupted 2026-10-29** — hold default (zero-slippage) output instead of 8 bps | re-emit with **`--sim-slippage-bps 8`** |
| sqrt rows | already drifted as of 2026-09-26, per `#123` — never fixed | re-emit with the declared sqrt flags |

### The trap I had not seen

Re-emitting the noop rows with the default invocation while leaving the canonical rows alone
gives both rows **the same body**. Two namespaces — the pre-friction oracle and "the current
reference for paper-trading alpha" (`anchors.toml:10-11`) — silently collapse into one. And
because `#151` changes every body anyway, both rows would just receive the same new SHA and
`verify_anchors.sh` would still print `119 / 119`. **The collapse is invisible to every gate
in the repo.** That is the single most dangerous thing in this pass, and it is not something
the pass would have told me about.

## What was executed — operator-ruled "full repair, one pass"

### The classification, final

| bucket | rows | disposition |
|---|---|---|
| **D6B** | 24 | re-emitted; figures moved, each cause quantified |
| **D6C** | 13 | re-emitted; arm's figures byte-identical, content only |
| **ORACLE** | 28 | **not touched** — frozen pre-friction baselines, unreachable from current code by design |
| **UNRUN** | 54 | not touched — rendered by `crates/forecast`'s sharpe-comparison and verdict bins, `crates/reports`, and the threshold-sweep bin, none of which calls a changed writer |
| | **119** | all accounted for |

Zero rows carried the `ALL n shared figures moved` warning, which is what makes the 24 D6B
rows credible as drift rather than as runs under a wrong condition.

### Placement, and the invariant that protects the oracles

Each new body went into **the directory that already serves its row**, as reported by
`verify_anchors.sh --explain` — the authoritative resolver, rather than my reading of its
`find | sort | tail -1` logic. The fresh bodies are already named `backtest-20261108-*`, which
sorts after every superseded name under `LC_COLLATE=C`, so resolution picks them without any
renaming. Nothing was overwritten: 37 *new* files, asserted non-existent before the copy.

One invariant was asserted per row before anything was written, and it is the one that could
have destroyed something silently:

> A `noop-baseline` row resolves to the newest body **outside** the
> `v5-latency-slippage-sim` migration dirs. 28 such rows are frozen oracles I am deliberately
> not re-emitting. If a **canonical** body landed outside those dirs it would become the
> newest-outside body for its scenario and hijack that scenario's oracle row — destroying the
> pre-friction baseline while every gate stayed green.

So: canonical and sqrt bodies must land INSIDE a migration dir, noop and legacy bodies
OUTSIDE. Both directions asserted; all 37 passed.

### Verification

- **exactly 37** `sha256` lines changed in `anchors.toml`; **82 unchanged** — the § D6.b step 5
  / § D6.c step 6 negative invariant, counted rather than claimed.
- **exactly 37** rows now resolve to a `20261108` body under `--explain`; 119 rows PASS.
- `scripts/verify_anchors.sh` → **ANCHORS PASS (119 / 119)**.
- `scripts/anchor_namespace_collapse_check.py` → **PASS, all partitions intact** — the
  namespaces did NOT collapse, which was the specific danger of this pass.
- Every row carries a comment block naming its protocol section (§ D6.b or § D6.c), its
  **declared condition** (which invocation produced it), its cause, its measured deltas, the
  superseded body filename, the superseded SHA verbatim, and this record.

### System 2 re-locked in the same pass (§ D6.c step 7 / ADR-0045 § D6.1)

`check_determinism_anchors.py` → **OK, 35 of 35 resolved literals match** (12 canonical, 17
declared-ns, 6 synthetic; 0 skipped, 0 unresolved). Getting there required fixing the tool
twice — it could detect a stale pin but not repair one, and once taught to, it wrote three
wrong values. That is bug-log `#180`.

## The proof

```
cargo test -p backtest --test determinism --features candle,realdata
test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 324.18s
```

**Green for the first time since 2026-10-17.** The sequence, because the middle step is the
interesting one:

| run | result | |
|---|---|---|
| before the pass | 29 passed · **10 failed** | the drift `#177` found |
| after re-emission + re-lock | 33 passed · **6 failed** | the ten fixed — and six DIFFERENT ones now red |
| after `#181` | **39 passed · 0 failed** | — |

Those six were not a regression from the repair; they were a **third pin location** the repair
made visible. `SYNTHETIC_DETERMINISM_SHAS` holds SHAs for three bodies that exist nowhere on
disk, and the consistency checker compares the in-test literal against that map — a copy
checked against its own copy, which always passes. `#151` moved those bodies and nothing could
report it. Re-locked with the cause established four ways (see `#181`), including a
**second, independent invocation** (`--force-synthetic-bars`) that reproduces the values to
the byte — so the pin is now backed by a declared invocation rather than a tempdir side
effect.

## Final state

| gate | result |
|---|---|
| `scripts/verify_anchors.sh` | **ANCHORS PASS (119 / 119)** |
| `scripts/anchor_namespace_collapse_check.py` | **PASS** — 36 multi-row scenarios, all partitions intact |
| `scripts/check_determinism_anchors.py` | **OK — 35 of 35** (12 canonical · 17 declared-ns · 6 synthetic · 0 unresolved) |
| `cargo test --test determinism --features candle,realdata` | **39 passed · 0 failed** |
| `scripts/spec_lint.py` (+ `--self-test`) | **PASS** |

37 new bodies, every one carrying the buy-and-hold control, none truncated; 82 anchor rows
byte-identical; 28 frozen oracles untouched.

## What the control says, now that there is one

The first thing `#151`'s benchmark does on real data is answer the question the corpus could
not:

| arm | its return | buy-and-hold, same window | gap |
|---|---|---|---|
| `top10-2023-fy-tcn-overlay-weights-realdata` | −27.63% | **+196.10%** | **−223.73 pp** |
| `btc-2023-1m-macd-trend` (synthetic) | −97.42% | **+42.45%** | **−139.87 pp** |

Both bodies printed their own return for their entire existence with nothing beside it. This
does not change the era-qualified ship-passive thesis — that rests on the θ-surface corpus,
where buy-and-hold already *is* the control and 0 of 156 cells beat it — but it is the first
time a LANE body says the same thing on its own.

## POSTSCRIPT — the pass missed two rows, and CI said so

`fc76fff3` took the **ubuntu** leg red. One leg, which is the clue: the ADR-0097
reproducibility gate is ubuntu-only and `reproducibility_sample_figure.rs` carries
`#![cfg(feature = "yahoo")]`, which only that leg enables.

**Two yahoo rows were missed, and the bucket name is the defect.** `report/sma.rs` is shared
by the yahoo path (`bin/run_yahoo_sma` → `report/yahoo.rs` → `report/sma.rs`), so `#151` moved
those bodies too. The classifier filed them `UNRUN`, which this very note described as "no new
body … **unchanged by construction**". The first half was true; the second does not follow:

- rendered by code not in the diff → genuinely unchanged;
- rendered by a CHANGED writer via a binary the sweep did not run → **changed and invisible**.

§ *How the bodies were produced* above says the sweep drove `target/release/backtest` only —
and § *D6.c step 2* above correctly lists `bin/run_yahoo_sma.rs` as a caller of the changed
writers. The right fact was two sections above the bucket that discarded it.

Repaired under § D6.c: both rows re-emitted with their declared invocation, figures
byte-identical (the diff is exactly the two added rows), three test constants re-locked, all 7
yahoo tests green. **Exactly 2 `sha256` lines changed; 117 of 119 untouched.** btc-yahoo
reproduces identically from the committed `data/yahoo-sample` slice and from the full
`data/yahoo` cache, so ADR-0097's premise was re-verified by the repair rather than assumed
through it.

The class is closed in the tool: `UNRUN` now splits into `UNRUN` and **`NOT-COVERED`**, the
latter driven by a declared `EXTERNALLY_RENDERED` map that names each scenario *and the command
that produces it*, printed on every run and **exiting non-zero**. A sweep can no longer be
silent about what it did not cover. Bug-log [`#182`](bug-log.md).

**Running total for this pass: 39 of 119 rows re-emitted** (37 + these 2), 28 frozen oracles
untouched, 52 rows rendered outside any changed writer, 2 rows now permanently flagged as
needing their own binary.
