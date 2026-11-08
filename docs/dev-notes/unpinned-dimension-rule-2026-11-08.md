# The unpinned-dimension rule — 2026-11-08

Five defects landed on 2026-11-07/08 (`#156`, `#158`, `#159`, `#160`, `#161`). They were found
by seven different search angles and they live in four different crates. They are one defect.

This note states the shape mechanically, because the house rule as usually stated — *"a gate
that cannot fail is indistinguishable from a gate that passes"*, the thread running through
fourteen [bug-log](bug-log.md) entries — is true but not **checkable**. It tells you what you are looking
for; it does not tell you where to look. Each of the five was found by a different ad-hoc
argument, and a 61-agent sweep still missed the worst one
([`cannot-fail-gate-sweep-2026-11-07.md`](cannot-fail-gate-sweep-2026-11-07.md) § 4(a)).

## The rule

> A constant, term or branch in a gate is **pinned** only if the fixture set contains two cases
> that differ in that dimension and whose expected outcomes differ.
>
> Otherwise the fixture set has **zero variance** in the dimension that constant controls, and
> the constant is untested — regardless of how many fixtures there are, how green they are, or
> how directly they name it.

The test-count is not evidence. `#161` had 368 green tests over the function it failed to pin.

## The five, in the rule's terms

| | the dimension | why the fixtures had no variance in it |
|---|---|---|
| `#156` | identity 4's two sides | both sides were the **same expression**, so no input could separate them |
| `#158` | `period_end` reaching the body | the equity curve was a **constant**, so the body could not move when the clock did |
| `#159` | `unrealized` in identity 4 | both fixtures held **no open positions**, so the term was 0 in every case |
| `#160` | `ENTROPY_THRESHOLD` | the one firing fixture sat at the **arithmetic maximum** of the measure, so no raise could cross it until 5.2 |
| `#161` | 4 of 5 ROBUST bands | every negative fixture failed on the **first** conjunct, so the other four never decided an outcome |

Read down the right-hand column: *same expression*, *a constant*, *always zero*, *at the
maximum*, *never reached*. Five ways of saying the fixtures did not vary the thing under test.

## Why "cannot fail" under-describes it

"Cannot fail" suggests a gate that is inert. Four of these five **did** fail, correctly, on the
dimensions they *were* pinned on:

- `#160`'s redactor caught every `sk-ant-` key the regex rules knew.
- `#161`'s classifier caught every FRAGILE candidate, and caught `P5_SHARPE_ROBUST`.
- `#156`'s table caught identities 2 and 3 — the single FAIL-path test in the tree breaks
  exactly those two.
- `#159`'s replacement row caught fees, which is what it was built for.

So each one was a **partially** pinned gate, reporting the dimensions it covered and silent on
the rest — which is far more convincing than an inert gate, and why they survived. `#156` is the
extreme: it published `Δ = 0 | PASS`, an actively reassuring sentence, over 5.17 USDT of
unaccounted fee spend.

## The mechanical check

For each constant or conjunct `C` in a gate, ask: **is there a fixture pair differing only in
`C`'s dimension, with different expected outcomes?**

That question is answerable by perturbation, and perturbation is cheap:

1. Copy the file (or crate). Never perturb the repo — see `#161`'s probe discipline.
2. Move `C` in **both** directions, one at a time.
3. Run the narrowest scope that includes every test that could see it.
4. **Verify the file actually changed** before believing the result. A probe that silently
   no-ops reads exactly like a gate that does not bite — that happened while fixing `#160`:
   a shell loop split its patterns on `:`, so four perturbations ran unmodified code and
   reported `0 FAILED`.

If neither direction goes red, `C` is unpinned. That is a finding, and it is a *measurement*,
not an argument.

### What the fix looks like, every time

A **two-sided pair per dimension**, with everything else held clear by construction:

- one fixture exactly **at** the boundary (pins the inclusive/exclusive comparison, and catches
  the constant being tightened),
- one **just outside** it (catches the constant being loosened),
- and the baseline asserted to land where intended first, so a drifted baseline fails loudly
  instead of making every case compare against an outcome reached for another reason.

Derive the fixture's value from a closed form rather than measuring it off the subject. `#160`'s
entropy fixtures use `H = log2(n) - 2a/n`; the assertion they replaced was
`assert!(shannon_entropy(FIXTURE) >= ENTROPY_THRESHOLD)`, whose expected value **was the shipped
constant** and which therefore held for any threshold at or below the fixture's entropy.

## Where it is already mechanical

`scripts/verdict_transcription_check.py` (ADR-0099, `#154`) does this for **branch conditions**
in three forecast classifiers: it pins each classifier's condition set against a set declared in
the script, so both-sides equality is not enough. Note why the declaration matters — a
normaliser bug that mangled `.max(1e-12)` into `.max1e-12` identically on both sides passed the
equality check and was caught only by the declared set.

The gap this note names is that the same discipline is not applied to **thresholds and
conjuncts**, only to branch conditions, and only in three files. `#160` and `#161` are both
outside that gate's table.

## Scope limits, stated

- This is a rule for **gates** — code whose output is a verdict, a label, a redaction decision
  or a reconciliation. It is not a general test-design principle and should not be read as one.
- It says nothing about whether a constant's **value** is right. Every fix above pins the value
  that ships; none argues for changing one. Moving a band is a separate, operator-level act
  (AD-1 freezes four functions outright).
- A latent gap is still a gap. `#161` relabels nothing in today's corpus (784 FRAGILE / 80
  MARGINAL / 0 ROBUST, max p5 0.182983), which is precisely why no anchor could catch it. The
  anchors measure what the gate *did*; this rule measures what it *would do*.
