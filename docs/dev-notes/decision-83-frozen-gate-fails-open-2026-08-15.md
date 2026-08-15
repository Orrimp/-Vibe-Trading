# Decision memo — bug-log #83: the frozen gate fails open

**For**: operator · **Prepared**: 2026-08-15 · **Needs**: an AD-1 ruling (the fix touches a FROZEN
file) · **Status**: staged, NOT applied.

This is a yes/no, not an investigation. Everything below is verified at source.

---

## 1. What is wrong

`rank.rs::is_eligible` — the rule that decides which candidates may be crowned:

```rust
fn is_eligible(c: &CandidateResult) -> bool {
    c.is_benchmark || c.robustness != Some(RobustnessFlag::Fragile)
}
```

It is a **deny-list**: everything that is not `Fragile` is eligible. `Some(RobustnessFlag::Skipped)`
— which `compute_robustness_flag` returns when the robustness computation **could not run** — is
therefore crown-**eligible**. A gate that failed to evaluate counts as a gate that passed.

## 2. The part that makes this an easy ruling

**The correct treatment already exists in this codebase**, at the other call site of the very same
function. `sweep.rs:1159-1174` handles the identical `None` from `compute_robustness_distribution`
by mapping it to **Fragile**, and states why:

> *"Curve too short — treat as Fragile (consistent with `Skipped`→Fragile in the leaderboard
> context; a curve too short to score is untrustworthy)."*

**That parenthetical is false.** In the leaderboard context `Skipped` is *eligible*, which is the
opposite of Fragile. So this is not "should we adopt a new policy?" — it is **two call sites of one
function disagreeing, with the safer one already written and reasoned, and the riskier one carrying
a comment that asserts they agree.** Same shape as every other defect this session: a documented
belief that nothing checks.

## 3. The good news that makes the fix small

`Skipped` looked overloaded — "deliberately not run" vs "could not run" — which would have made this
hard. It is not. The two states are **already distinct**:

| situation | value | should it be eligible? |
|---|---|---|
| operator chose `RobustnessMode::Skip` | `c.robustness == None` (mod.rs:1332, 1419) | **yes** — deliberate |
| gate ran and could not score the curve | `c.robustness == Some(Skipped)` (bootstrap.rs:220) | **no** — this is the bug |

So the fix needs **one file, one function**. `bootstrap.rs` does not change at all; `compute_robustness_flag`'s only production caller is `mod.rs:1341`.

## 4. The two candidate patches

**Option A — minimal (deny-list, add `Skipped`):**
```rust
fn is_eligible(c: &CandidateResult) -> bool {
    c.is_benchmark
        || !matches!(c.robustness, Some(RobustnessFlag::Fragile | RobustnessFlag::Skipped))
}
```

**Option B — durable (allow-list) — RECOMMENDED:**
```rust
fn is_eligible(c: &CandidateResult) -> bool {
    c.is_benchmark
        || matches!(
            c.robustness,
            None | Some(RobustnessFlag::Robust | RobustnessFlag::Marginal)
        )
}
```

Both produce identical behaviour today. They differ on the **next** variant someone adds to
`RobustnessFlag`: under A it is silently crown-eligible (the same failure again); under B it is
ineligible until someone deliberately adds it. B fails closed, and the compiler points at the
decision. A gate whose default answer is "pass" is the thing being fixed — so the fix should not
itself default to "pass".

## 5. Severity — stated honestly, because I could not settle it

**I could not determine whether this has ever fired.** The reason is worth knowing on its own:
**none of the 62 anchored reports render the robustness flag** — `grep -i` for `robust|fragile|
marginal` across `evidence/*/reports/*.md` returns **zero** hits. The FROZEN gate's own verdict, the
value that decides crown eligibility, is not witnessed anywhere in the anchored corpus, so the corpus
cannot answer the question.

What can be said structurally: `Some(Skipped)` requires an equity curve of **fewer than 2 points**
(`bootstrap.rs:128-136`), which a normal backtest will not produce. So this is *probably* latent
rather than live — but "probably" is doing real work in that sentence, and a candidate that fails to
run at all is exactly the case bug-log **#78** already caught in the ranked field once.

## 6. Verification plan for whoever applies it

1. **Identity test** — assert the ranking of every existing candidate set is unchanged when no
   candidate carries `Some(Skipped)`. This is the AD-1 identity obligation ("prove it does not change
   ranking"), and it should be a *new* test file, not an edit to a frozen one.
2. **RED-proof** — construct a candidate with a 1-point equity curve, confirm it is crowned **before**
   the patch and excluded **after**. Without this the patch is unwitnessed.
3. **Anchors 119/119** before and after — expected to hold trivially, since §5 indicates no anchored
   run carries the flag.
4. Consider making the sweep/leaderboard agreement **structural** rather than commented, so the two
   call sites cannot drift again — otherwise the next reader inherits the same false parenthetical.

## 7. What I did not do, and why

I did not apply the patch. `rank.rs` is byte-frozen under **AD-1** ("`rank_candidates` is not edited
by feature work"), and this is a change to crown eligibility — the gate's whole purpose. It needs
your ruling, not my judgment. I also did not correct the false comment in `sweep.rs`, because
touching it before the ruling would erase the evidence that motivates it.
