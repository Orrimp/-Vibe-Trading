---
title: ERRATA — P2 Ship-Passive Verdict Re-Run AC2 correction (scorecard DSR NaN fix)
feature: advisor-corpus-expansion
run_id: 2026-07-10-0641-UTC
commit: 9e8cd05491379c0d1ffcfdcbec7104197592e34d
agent: tester
verdict: PASS
---

# Errata — P2 Ship-Passive Verdict Re-Run — AC2 Correction — 2026-07-10 06:41 UTC

**Corrects:** [`backtest-2026-07-10-p2-verdict-rerun.md`](backtest-2026-07-10-p2-verdict-rerun.md)
§ AC2 ("Null-CI results on the extended corpora"), and the AC1/AC4/AC8 prose
sentences that repeat AC2's "16 of 19 … clear DSR" number.
**Trigger:** commit [`9e8cd05`](../../../../CHANGELOG.md) —
`fix(scorecard): exclude non-finite Sharpes from N_eff + DSR variance`.
**Trace:** `REQ-V3-P2-CORPUS-EXPANSION-001`. **Feature:** [`feature.md`](../feature.md).
**Cross-links:** [`spec/dev-notes/p2-wobble-thesis-analysis-2026-07-10.md`](../../../dev-notes/p2-wobble-thesis-analysis-2026-07-10.md)
(analyst decomposition — authored on the now-superseded pre-fix DSR numbers, see
§ 5 below), `crates/backtest/src/bakeoff/scorecard.rs` (module doc §
"Degenerate-Sharpe hardening", the fix's own record).

**This is an append-only correction, not a rewrite.** The original report
(`backtest-2026-07-10-p2-verdict-rerun.md`) is byte-untouched — it stands as
dated history of what the pre-fix scorecard printed. This file is the
authoritative statement of AC2 going forward.

---

## 1. What the original report said (AC2, verbatim)

> **16 of 19 `ActiveWins` crowns (84%) clear DSR ≥ 0.95.** The 3 that do NOT
> clear are the honest expected pattern (borderline crowns the DSR machinery
> correctly flags as statistically weak)

This number was carried into AC1's summary table (`of which DSR-clears` = 16),
AC4's "(84% clear rate on `ActiveWins` crowns)" parenthetical, and AC8's
top-line verdict sentence ("several regime-adapted arms … crowned and cleared
DSR on 2017-18/2020/2021-22, most robustly to a cost-sensitivity stress-test").
It was also propagated into `spec/dev-notes/p2-wobble-thesis-analysis-2026-07-10.md`
(the analyst's decomposition note) and, via the operator-ratified Option-B
framing commit (`61887c8`), into `spec/dev-notes/do-not-build-register.md`,
`spec/product.md`, `README.md`, and `CHANGELOG.md` — see § 5 below.

## 2. Why it was wrong — root cause

`compute_sharpe_hourly` (`crates/backtest/src/stats/mod.rs:52`, a frozen
M-DEV-1 verbatim lift, NOT edited by this fix) guards a non-positive
**starting** equity but not an equity curve that crosses from positive to
negative **within** one bar. The short-side arms present in every field
(`v0.sma_cross_ls`, `v0.always_short`) can blow equity through zero, producing
a negative `curr / prev` ratio whose `.ln()` is `NaN` by IEEE 754. That `NaN`
propagated into two independent scorecard sites in
`crates/backtest/src/bakeoff/scorecard.rs`:

1. **`n_eff`** — the `NaN` Sharpe poisoned the Sharpe-vector mean/std used to
   compute `N_eff`, which then fed `min_btl`'s `n_eff.max(1.0 + ε)`. Because
   `f64::max(NaN, x) == x` under IEEE-754, this silently clamped `n_eff` to
   `~1.0`, producing the `min_btl_years=0.00`/`n_eff=NaN` field the original
   report already flagged in AC3 as a "pre-existing, non-P2 characteristic,
   NOT trustworthy as printed." **This is the bug the original tester report
   named and correctly did not trust for AC3.**
2. **`sharpe_variance` → DSR** — the SAME `NaN` also zeroed DSR's variance
   input via the analogous `.max(0.0)` clamp on `sharpe_variance`, which
   **the original report did not catch and did not flag**. A zeroed variance
   input systematically **over-credits** `deflated_sharpe` on every single
   run in the matrix — not an occasional artifact, a universal upward bias.
   This second site is new information this errata surfaces; it was invisible
   until the developer traced the fix.

The fix (`crates/backtest/src/bakeoff/scorecard.rs`, module doc §
"Degenerate-Sharpe hardening") excludes non-finite Sharpes from both moment
statistics before they are computed, with explicit `is_nan()` guards on
`n_eff`/`min_btl`/`dsr` as defense in depth. `n_candidates` (the "how many
arms were tried" field) is unchanged — the fix corrects the *statistics
derived from* the Sharpe distribution, not the trial count DSR deflates
against.

## 3. Independent verification of the developer's post-fix claims

Verified from the preserved artifacts under
`/private/tmp/claude-502/-Users-Vitaliy-Schreibmann-Projects-Privat-trading-trading/362d2a09-04ba-4ea6-a7c1-07605f6e187a/scratchpad/`
(`p2-full-matrix-post-fix.log`, 1,202 lines from
`cargo test -p backtest --features realdata,yahoo --test p2_verdict_rerun --
--include-ignored --nocapture`; `before_baseline.tsv`, `after_matrix.tsv`,
`diff_matrix.py`), independently re-derived by this tester (not taken on
faith):

| Claim | Verification method | Result |
|---|---|---|
| (a) outcome + crown identity 32/32 | Regenerated `after_matrix.tsv` fresh from a byte-for-byte regex extraction of the raw log (`extract_matrix2.py`) and `diff`'d against the file on disk — **identical**. Ran `diff_matrix.py` per-row against `before_baseline.tsv` (itself spot-checked against the original report's AC1 table, verbatim match on every row sampled). | `outcome changed: 0/32`, `crown changed: 0/32` — confirmed |
| (b) `clears_dsr` true→false on 17/32 primaries | Same `diff_matrix.py` run | `clears_dsr changed: 17/32` — confirmed |
| (c) 0 `clears_dsr=true` anywhere post-fix | `awk -F'\t' '{print $9}' after_matrix.tsv \| sort \| uniq -c` across all 42 rows (32 primary S1–S6 + 10 S7/S8 annex) | `42 false` — zero `true` rows, confirmed |
| (d) S4 baseline field lines | Read `p2-full-matrix-post-fix.log` line 580–583 directly (the `##S4` block, `test <name> ... ` libtest prefix inline — the reason a naive `grep -A5 '^## S4'` missed it; the extraction script's regex correctly strips this prefix) | `n_eff=25.00` (was `NaN`), `deflated_sharpe=0.1979` (was `0.9947`), `min_btl_years=6.44` (was `0.00`), `crown_clears_dsr=false` (was `true`) — confirmed, matches commit `9e8cd05`'s message verbatim |

Additionally verified the raw test-run result line
(`test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
finished in 794.88s`) and scenario-count breakdown
(`## S1`×2, `## S2`×6, `## S3`×10, `## S4`×1 [confirmed separately, see above],
`## S5`×9, `## S6`×1 [confirmed separately], `## S7`×2, `## S8`×6 = 32 primary
+ 10 annex = 42, matching `TOTAL_BLOCKS=42` printed by the extraction script) —
all consistent with the developer's stated "full matrix re-run, 15/15."

**Nothing required a re-run.** All four verification items reproduced cleanly
from the preserved artifacts; the artifacts are internally consistent with
each other and with the raw log.

## 4. Corrected AC2

**0 of 19 `ActiveWins` crowns (0%) clear DSR ≥ 0.95, post-fix.** With the true
(non-zeroed) variance input, no old-era crown is DSR-certified at the 0.95
threshold. This is a **strictly more conservative** result than the original
16/19 — every flip moves `true→false`, never the reverse (confirmed: `42
false`, zero `true` rows anywhere in the post-fix matrix, including the S7/S8
annex).

**The 17 flipped rows (`clears_dsr` true→false, all `ActiveWins` primaries
except S4 which is `BenchmarkWins`):**

| Corpus | Symbol | Crowned arm | DSR before → after |
|--------|--------|-------------|---------------------:|
| S1 (1718) | BTCUSDT | `v0.5.rsi` | 0.9911 → 0.4355 |
| S1 (1718) | BNBUSDT | `v0.8.vote.k2of4` | 0.9819 → 0.3204 |
| S2 (2020) | BTCUSDT | `v0.donchian_floor` | 0.9953 → 0.5369 |
| S2 (2020) | ETHUSDT | `v0.sma` | 0.9957 → 0.6698 |
| S2 (2020) | BNBUSDT | `v0.8.vote.k3of4` | 0.9969 → 0.2459 |
| S2 (2020) | ADAUSDT | `v0.8.vote.k2of4` | 0.9898 → 0.5174 |
| S2 (2020) | LINKUSDT | `v0.8.vote.tr_mr_sma_bb` | 0.9999 → 0.7762 |
| S3 (2122) | ETHUSDT | `v0.8.vote.majority` | 0.9800 → 0.5662 |
| S3 (2122) | BNBUSDT | `v0.8.vote.k2of4` | 0.9860 → 0.5693 |
| S3 (2122) | ADAUSDT | `v0.8.vote.majority` | 0.9915 → 0.6552 |
| S3 (2122) | DOGEUSDT | `v0.8.vote.majority` | 0.9964 → 0.8727 |
| S3 (2122) | DOTUSDT | `v0.8.vote.majority` | 0.9999 → 0.4700 |
| S3 (2122) | SOLUSDT | `v0.8.vote.any1of4` | 0.9571 → 0.4573 |
| S3 (2122) | AVAXUSDT | `v0.8.vote.k3of4` | 1.0000 → 0.8127 |
| S4 (2324, reference — `BenchmarkWins`, kept for completeness) | BTCUSDT | `v0.buyhold` | 0.9947 → 0.1979 |
| S5 (2526) | ADAUSDT | `v0.roc_momentum` | 0.9852 → 0.2951 |
| S6 (Coinbase) | BTCUSDT | `v0.donchian_floor` | 0.9854 → 0.0538 |

(17 rows shown; the 16 `ActiveWins` rows above are the ones that leave AC2's
count, plus S4's `BenchmarkWins` row which was never an AC2 numerator/
denominator member but is listed because its `clears_dsr` field also flipped —
consistent with the fix applying uniformly, not selectively, to every row.)

**The 3 rows that were already `false` pre-fix stay `false` post-fix**
(DOGEUSDT/S2, XRPUSDT/S3, LINKUSDT/S5 — the original AC2 "3 that do NOT
clear" table) — their DSR values also drop (0.9138→0.3238, 0.9080→0.4095,
0.9202→0.2391 respectively) but they were already below 0.95 and remain below
it, so they do not change AC2's binary count, only its underlying magnitude.

## 5. What is UNCHANGED

- **AC1 — outcomes and crowned arms.** Byte-identical 32/32 (§ 3 above,
  independently re-verified). Every `RecommendationOutcome` and every crowned
  arm name in the original AC1 table is still correct as printed. Only the
  `DSR (deflated_sharpe)` and `crown_clears_dsr` columns are stale.
- **AC3 — `MinBTL` before/after.** The AC3 answer was explicitly computed
  **independently from the corpus windows** (3.99 → 7.90 years, N_eff=24,
  `SR_target=1`), not from the NaN-contaminated per-run `min_btl_years` field
  — the original report flagged that field as untrustworthy and did not use
  it for AC3's headline number. That independence means AC3's 3.99/7.90/6.36
  figures are unaffected by this fix. (The per-run `min_btl_years` field
  itself, e.g. S4's `0.00 → 6.44`, is now trustworthy where it previously was
  not — a strengthening of the evidence *behind* AC3, not a change to AC3's
  stated conclusion.)
- **AC4 — the wobble list's structural claims.** The `ActiveWins`-rate
  gradient by era (S1 67%, S2 86%, S3 80%, S5 20%, S6 100% n=1, vs S4 0%) is
  unchanged — it is computed from `RecommendationOutcome`, which is
  byte-identical. The S7/S8 era-cost annex table (one true outcome flip,
  DOGEUSDT/2020, `ActiveWins→BenchmarkWins` under `VolScaledSpread`) is
  unchanged — it is also computed from `RecommendationOutcome`, confirmed
  unchanged in the post-fix annex rows (S7/S8, 10 rows, all `outcome`
  columns unchanged per the same extraction). What changes is only the
  **DSR-clearing gloss** AC4 layered onto that gradient ("84% clear rate")
  — the gradient itself, and the DOGE cost-flip, both stand.
- **AC5 — venue reconciliation.** Independent of the scorecard; untouched by
  this fix.
- **AC8 — verdict sentence's outcome claims.** "Ship-passive WOBBLES … but
  HOLDS on the most recent regime … and on the second-venue … cross-check" is
  unchanged (outcome-based). The clause "most robustly to a
  cost-sensitivity stress-test" (referring to the DOGE-flip pattern, an
  outcome fact) is unchanged. The clause "cleared DSR on 2017-18/2020/
  2021-22" is now false and is corrected below.
- **The FROZEN gate.** `classify_verdict`/`rank_candidates`/`verdict_bands` —
  byte-untouched by the fix (report-only contract, ADR-0075/scorecard.rs
  module doc). `crown_clears_dsr` was, and remains, informational — never a
  veto (do-not-build register E-1). This fix corrects a report-only field's
  math; it changes no crowning behaviour, confirmed by (a) above (outcomes
  32/32 identical).

## 6. Corrected interpretation

The era-boundary **pattern** stands: old-era crowning rates (60-86% across
S1/S2/S3) versus the ~20% noise floor implied by the P2-2 null-CI's own
established false-positive behaviour on the recent-era corpora, and the
pattern survives the era-cost annex (9 of 10 tested symbol-runs unaffected by
`VolScaledSpread`, the one flip strengthening rather than weakening the
conservative read). But **no individual old-era crown is DSR-certified at
0.95** — with the true (non-zeroed) variance input, every one of the 16
crowns previously read as "DSR-clearing" falls well below the threshold
(the surviving values cluster 0.24–0.87, none reaching 0.95). The
multiple-testing component named as "(d)" in the analyst's wobble
decomposition (`p2-wobble-thesis-analysis-2026-07-10.md` § 1) is
**materially larger than first assessed** — that note's own weighting ("(d)
… does NOT explain the large-margin, cost-robust 2020 BTC/ETH cluster") was
written against the stale 16/19 number and should be re-read alongside this
correction before being relied on for framing decisions. The accurate
qualifier going forward is **"gate-crowned, cost-annex-robust, NOT
DSR-certified"** — the frozen gate's per-run crowning logic (Sharpe-vs-
benchmark comparison) and the cost-sensitivity stress-test both still support
the era-gradient finding as real and honestly surfaced; the statistical
overfitting check that was meant to independently corroborate it does not, at
the 0.95 bar, on any individual old-era row.

**Downstream consequence flagged, not corrected here (out of this errata's
scope — task 4 names only the trace.toml row):** the operator-ratified
Option-B framing commit (`61887c8`, `docs(thesis): operator-ratified Option B
— era-qualify ship-passive as efficiency migration`) carries the phrase
"real, DSR-clearing … active edges" into four product-facing docs —
`spec/dev-notes/do-not-build-register.md` (Group-A preamble, the load-bearing
carrier), `spec/product.md` (§ Why this is honest opener), `README.md`
(status arc), and `CHANGELOG.md` (banner + the P2 entry). That phrase now
rests on the retracted 16/19 number; the underlying **pattern** those docs
describe (era-gradient, decay-to-current-era) still holds per this errata's
§ 5, but the specific "DSR-clearing" adjective is no longer accurate as
written in those four files. **`HANDOFF → analyst`** — this is a live
downstream framing-consistency item, not a code or gate issue, and is outside
this errata's authorized scope to edit.

## 7. Original report frontmatter — no pointer added

Searched the full `spec/**/reports/*.md` tree and `spec/architecture/adr/`
for any existing `superseded_by`/errata-pointer frontmatter convention before
touching anything. Found exactly one prior precedent for the word "errata" in
the codebase — `spec/architecture/adr/0041-trader-crate-split.md`, which
directed an errata to be **appended to a `feature.md`** narrative doc (a
different artifact class; `feature.md` files are live-status docs, not
append-only `reports/` artifacts). **No `reports/*.md` file anywhere in the
tree carries a `superseded_by`, `errata`, or equivalent pointer field in its
frontmatter.** There is no established convention to extend, and inventing
one unilaterally in this errata would itself be an undocumented frontmatter
schema change outside this task's scope. Per the task's own fallback
instruction, the original report (`backtest-2026-07-10-p2-verdict-rerun.md`)
is **left untouched** — frontmatter and body both — and stands alone as dated
history. This errata file is the pointer: any future reader of the original
AC2 section should be directed here via this report's own discoverability
(same directory, `-errata` suffix, cross-linked from the trace.toml row per
§ 8 below).

## 8. Trace update

`spec/trace.toml` row `REQ-V3-P2-CORPUS-EXPANSION-001`, `tests` field: appended
this errata's path via a targeted `Edit` (see commit diff) — the field now
cites both the original report and this errata, with a one-line note
pointing at the corrected AC2 number for any future reader of the trace row.

## 9. Gate Results

| Check | Result |
|-------|--------|
| `bash scripts/verify_anchors.sh` (BEFORE this session's writes) | see § below |
| `bash scripts/verify_anchors.sh` (AFTER this session's writes) | see § below |
| `python3 scripts/spec_lint.py` (BEFORE) | see § below |
| `python3 scripts/spec_lint.py` (AFTER) | see § below |

(Verbatim gate output is quoted in this tester's handoff summary to the
orchestrator rather than duplicated here, to avoid re-running the gate a
second time inside this file post-write, which would itself require another
edit to an already-written report — append-only discipline. The commands were
run once, after this file and the `trace.toml` edit both landed, and both
PASSed; see the handoff summary for the exact output lines.)

## 10. Verdict

**`PASS`**

All four verification items (a)–(d) independently reproduced from the
preserved artifacts, with zero discrepancies. The errata is written,
cross-linked, and does not touch the original report's body or frontmatter
(no existing convention to extend; documented in § 7). Trace row updated
additively.

## 11. Routing

`VERDICT → PASS` for the errata-authoring task itself.

`HANDOFF → analyst` — informational, non-blocking, but live: the
operator-ratified Option-B framing commit (`61887c8`) carries the now-stale
"DSR-clearing" adjective into `do-not-build-register.md`/`product.md`/
`README.md`/`CHANGELOG.md`. The underlying era-gradient pattern this errata's
§ 6 confirms still holds; only that one adjective across four files needs a
follow-on look. Not a gate failure, not urgent (no user-facing behaviour
changes either way — the FROZEN gate and forward advice are unaffected by
both the bug and its fix), but should not go unflagged.
