---
adr: 0098
title: The cross-run multiple-testing annex
status: accepted
date: 2026-09-25
supersedes: none
superseded-by: none
---

# ADR-0098: The cross-run multiple-testing annex

## Context

The P0-1 overfitting scorecard (ADR-0075) deflates the crown's Sharpe by how many arms
were tried **in that run**. It cannot see the other axis. An operator who re-runs the
bake-off on a new coin, a new window, or simply a later date is running a *sequence* of
tests, and the chance that at least one of them says "beats holding" grows with the
length of the sequence **even when nothing ever beat holding**.

`research/backtesting/papers.md` [73] (Ramdas et al. 2017) names the tool: online-FDR /
alpha-investing over the run sequence, with decaying memory so a 2021 discovery does not
spend 2026's error budget. The 2026-07-11 gap analysis lists this as **the single
build-candidate** in the 900-paper corpus — everything else there is stated-limit or
leave-alone. The operator decided BUILD on 2026-07-27 (PRD §13 Q3), "as a report-annex".

## Decision

**D1 — Ship the static family-wise version, and say so.** Two numbers over the recorded
sequence:

- the **expected** number of false "beats holding" verdicts, `N × α`, which is what B7
  names as the minimum report-annex line;
- the **Šidák** per-run bar, `DSR ≥ (1−α)^(1/N)` — how high one run's deflated Sharpe
  must reach for the whole sequence to hold family-wise error at α — and how many
  recorded crowns clear it.

**LORD alpha-investing with decaying memory is NOT shipped.** It is the better fit for
crypto non-stationarity and needs wealth accounting across the sequence (a long
nothing-beats-hold streak *tightens* the budget, which is the correct self-skeptical
response). AC2 explicitly permits the cheap version for v0.1 **provided the choice is
stated**; it is stated in the module docs, in the rendered caption, and here.

The difference matters in one direction worth naming: Šidák assumes independence and
treats every run as equally current, so it is **conservative about old runs** and
**optimistic about correlated ones** — re-running the same coin on an overlapping window
is not a fresh test. Neither error is hidden; both are in the caption the operator reads.

**D2 — α is not a new knob.** `scorecard::DSR_THRESHOLD` is 0.95, so the crown's implied
per-run level is already 0.05. The annex uses the same one, or it would be describing a
different test than the one that ran.

**D3 — The ledger is git-ignored, and that is the anchor argument.**
`advisor-runs/fdr-ledger.jsonl`, a new sibling of `/lab-runs/` and `/plan-exports/`
(ADR-0055 precedent). Git-ignored ⇒ outside every `evidence/**` anchor glob ⇒ the 119
anchored bodies are byte-immutable **by construction**, not by anyone remembering. Rows
carry no money and no equity curve — they count tests, so AD-9 is not implicated because
there is no `Decimal` here to get wrong.

**D4 — Recording is an explicit `Option` at every construction site.**
`BakeoffRequest::fdr_ledger: Option<PathBuf>`, with no `Default`, so all 17 call sites
state their intent. `None` in every test and on the anchored CLI path; `Some` only on the
advisor path, because **that** run is the operator asking a question. A run that recorded
itself by accident would inflate the very denominator the annex reports — the sequence
would look longer than the operator's actual search, which biases the annex *pessimistic*
about their luck and, worse, makes the number meaningless.

**D5 — Read before append.** The annex describes the sequence **up to** this run; this run
joins the denominator only for the next one. That is the honest reading of "how many
tests had I already run when I got this answer".

**D6 — Damage is counted, never swallowed.** Unreadable ledger rows are reported in
`unreadable_rows` and force `Insufficient::PartiallyUnreadable`. A ledger that silently
dropped half its history would understate the sequence length, which biases the annex
**optimistic** — the one direction an honesty surface must never fail in. A missing file
is not damage: the first run is normal.

**D7 — `beats_hold_within_chance()` returns `Option<bool>`, not `bool`.** Insufficient
history yields `None`. "We cannot say" and "no, it is not within chance" are different
answers, and conflating them is how an honesty surface starts lying.

**D8 — REPORT-ONLY, proven over four ledger states.**
`crates/backtest/tests/fdr_annex_identity.rs` asserts the crown **and the full ranking
order** are identical with the annex absent and present across `{missing, empty,
populated, corrupt}` — each reaches a different branch of `read_ledger`, and a
report-only claim that held for one state but not another would be worth nothing.
Comparing only the winner would pass on a change that reshuffled everything below first
place, so `ranked` is compared in full. `robustness.rs` and `rank.rs` are byte-untouched.

**D9 — One clock, at the append site only.** `compute_annex` is pure and clock-free so the
arithmetic is testable without freezing time; the run-date label is read at the append
call and reaches only the git-ignored row, never a report body.

## Alternatives considered

- **LORD with decaying memory now** — the right answer eventually, deferred under AC2's
  explicit permission. Building wealth accounting badly would be worse than shipping the
  conservative bound and naming it.
- **Count arms rather than runs as the sequence** — rejected: the per-run scorecard
  already deflates by arms. Counting them again here would double-count the axis that is
  already handled and leave the one that is not.
- **Put the ledger under `evidence/`** — rejected. It would land inside the anchor globs
  and make a bookkeeping write a potential evidence mutation, which is the accident D3
  exists to make impossible.
- **Derive the ledger path from the process CWD instead of the request** — rejected: a
  cwd-relative corpus root is exactly what made the `ui` real-data guards vacuous for
  months (bug-log #66).

## Consequences

- 17 `BakeoffRequest` construction sites gained an explicit `fdr_ledger`; 15 are `None`.
- The advisor bake-off now writes one JSONL row per run to a git-ignored directory.
- **The annex is honest about being conservative.** With a short sequence the Šidák bar is
  barely above the per-run bar and the expected-false count is small; the annex will
  usually say "your beats-hold count is within what chance predicts", which for this
  product is the expected and correct answer rather than a disappointment.
- **Moral**: a per-run honesty statistic can be exactly right and still mislead, if the
  operator runs it repeatedly. The scorecard answers "how many strategies did I try?";
  nothing answered "how many times have I asked?" until now.
