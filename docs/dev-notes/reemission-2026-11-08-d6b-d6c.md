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
