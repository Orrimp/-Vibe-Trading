# Story 4.13: advisor-online-fdr-annex

Status: done

<!-- Analyst-drafted 2026-07-29 (Mary). Operator-DECIDED build: PRD §13 Q3 answer, 2026-07-27
     ("build as a report-annex"). This is gap-analysis B7 — the single build-candidate in
     docs/dev-notes/research-gap-analysis-2026-07-11.md; everything else there is stated-limit/leave. -->

## Story

As the operator of the Honest Advisor,
I want a report-only cross-run multiple-testing annex (online-FDR / alpha-investing) beside the per-run scorecard,
so that the false-"beats-hold" rate accumulated across my whole SEQUENCE of bakeoff re-runs (new coin, new window, a later date — the P2 32-run family) is measured and displayed honestly, closing the one gap per-run DSR cannot cover, without the frozen gate moving a byte.

## Acceptance Criteria

1. **Durable cross-run ledger:** every completed bakeoff run appends one row (run-date label, symbol, window, arm count + `N_eff`, crown identity, crown DSR, beats-hold outcome) to an append-only ledger in a **git-ignored** state dir (the ADR-0055 `/lab-runs/` precedent: git-ignored ⇒ outside every `evidence/**` anchor glob ⇒ 119 anchored bodies byte-immutable BY CONSTRUCTION). Append never rewrites prior rows; a failed append warns via `tracing` and never fails the run.
2. **The annex computes what the gap analysis recommends:** an online-FDR / alpha-investing procedure over the run sequence (`backtesting[73]` — Ramdas et al. 2017, LORD family with decaying memory, on Foster–Stine alpha-investing; the decaying-memory variant is the corpus-recommended fit for crypto non-stationarity), surfacing at minimum the B7 report-annex line: **the expected false-"beats-hold" count at α over the N recorded runs** (a static Šidák/Holm over the crown set is the named cheap fallback, acceptable as the v0.1 formula if LORD wealth-accounting is deferred — state which shipped).
3. **REPORT-ONLY (register row E-1 honored):** the annex renders beside the existing scorecard surfaces (carried on `Recommendation` like `Scorecard` — the `bakeoff/scorecard.rs` P0-1/ADR-0075 additive-annex pattern), phrased per the C2 FWER-vs-FDR insight (family-level signal and per-crown uncertainty can BOTH be true). No field of it is read by `rank_candidates`/`classify_verdict`/`verdict_bands`/`compute_robustness_flag`; `robustness.rs` and `rank.rs` are **byte-untouched**.
4. **Identity-test obligation (CLAUDE.md FROZEN-gate non-negotiable):** a test proves crowning and full ranking order are identical with the annex present vs absent, and with ledger states {missing, empty, populated, corrupt} — mirror `crates/backtest/tests/short_enabled_byte_identity.rs`.
5. **Graceful degradation + floor:** missing/empty/corrupt ledger renders an honest "cross-run history: insufficient (N=…)" line, never an error, never blocks a run; `verify_anchors` 119/119 before AND after; `python3 scripts/spec_lint.py` PASS; clippy clean.

## Tasks / Subtasks

- [x] Ratify ledger location/format + annex math — **done 2026-09-25**, ADR-0098 (+ Registry row, atomic).
- [x] Ledger writer at bakeoff completion — **done 2026-09-25**.
- [x] Annex math module `bakeoff/fdr_annex.rs` — **done 2026-09-25**.
- [x] Render line beside the scorecard + pixel proof — **done 2026-09-25**.
- [x] Identity test (AC4) + degradation tests (AC5) + the arithmetic fixture — **done 2026-09-25**.
- [x] Gates: anchors 119/119, spec-lint, clippy, fmt.

## Dev record (2026-09-25)

ADR-0098.

### What shipped, and what did not — AC2 asks for this plainly

**Shipped: the static family-wise version.** Two numbers over the recorded sequence — the
expected false "beats holding" count `N × α` (B7's named minimum), and the Šidák per-run
bar `DSR ≥ (1−α)^(1/N)` with how many recorded crowns clear it.

**Not shipped: LORD alpha-investing with decaying memory.** It is the better fit for
crypto non-stationarity and needs wealth accounting across the sequence. AC2 explicitly
permits the cheap version for v0.1 provided the choice is stated; it is stated in the
module docs, in the rendered caption, and in ADR-0098 D1 — including the direction of the
error: Šidák assumes independence, so it is **conservative about old runs and optimistic
about correlated ones**. Re-running the same coin on an overlapping window is not a fresh
test, and the operator is told so.

### The decisions worth knowing

| | |
|---|---|
| α | Not a new knob — `DSR_THRESHOLD` is 0.95, so the per-run level is already 0.05 (D2). |
| Ledger | `advisor-runs/fdr-ledger.jsonl`, git-ignored ⇒ outside every `evidence/**` glob ⇒ anchors byte-immutable BY CONSTRUCTION (D3). No money in the rows, so AD-9 is not implicated. |
| Recording | `BakeoffRequest::fdr_ledger: Option<PathBuf>`, no `Default`, **all 17 sites state intent**. 15 are `None`. A run that recorded itself by accident would inflate the denominator the annex reports (D4). |
| Order | Read BEFORE append: the annex describes the sequence UP TO this run (D5). |
| Damage | Unreadable rows are COUNTED, never swallowed — under-counting biases the annex *optimistic*, the one direction an honesty surface must not fail in (D6). |
| "Within chance" | `Option<bool>`. Insufficient history is `None`, not `false` — "cannot say" is not "no" (D7). |

### AC4, the FROZEN-gate obligation

`crates/backtest/tests/fdr_annex_identity.rs` asserts the crown **and the full ranking
order** are identical with the annex absent and present across all four ledger states
AC4 names — missing, empty, populated, corrupt — because each reaches a different branch
of `read_ledger`. Comparing only the winner would pass on a change that reshuffled
everything below first place, so `ranked` is compared in full. The field is three
resolvable arms plus the benchmark, so the ranking is four entries long; a two-entry
ranking would be a weak reorder detector. `robustness.rs` and `rank.rs` byte-untouched.

## Dev Notes

- **Design question (proposed, architect ratifies): where do runs record their tested-hypothesis count?** Proposal: `advisor-runs/fdr-ledger.jsonl` at repo root, git-ignored (new sibling of `/lab-runs/` + `/plan-exports/` — same ADR-0055 anchor-safety argument), serde rows with Decimal-as-string. Alternative: a sub-path under the existing `/lab-runs/`. Per-run hypothesis count = the arm family actually ranked in that run (`all_sharpes.len()` / `N_eff` both recorded so the annex can use either; the gap analysis counts RUNS as the online sequence and arms-within-run as the per-run family).
- **What the dev-note actually recommends (cited, not invented):** B7 verbatim: "online-FDR / alpha-investing (`backtesting[73]`) controls the false-'beats-hold' rate across a *sequence* of re-runs; a static Šidák/Holm on the crown set is the cheap version… Rec: build-candidate — a report-annex line (the expected false-positive count at α over N runs), NOT a gate change." `research/backtesting/papers.md` [73] adds the decaying-memory rationale (a 2021-regime discovery shouldn't spend 2026's error budget) and the alpha-wealth mechanic (a long nothing-beats-hold streak tightens the budget — the correct self-skeptical response).
- **Do-not-build register check (mandatory): PASS — and register-ENDORSED.** The register's closing § "What IS still legitimately open" names exactly this annex as the gap map's one build-candidate. Designed around **E-1**: no crown-eligibility veto, no `rank.rs` read of any annex field — the annex is informational exactly like `crown_clears_dsr`. Wiring it into eligibility later would require the full E-1 four-step bar (`docs/dev-notes/dsr-report-only-decision-2026-07-09.md`) — out of scope here and stated so.
- Era-qualified thesis unaffected: the annex measures selection pressure across runs; it neither adds alpha surface nor restates the thesis.

### References

- ADR: [`0098-the-cross-run-multiple-testing-annex.md`](../planning-artifacts/architecture/decisions/0098-the-cross-run-multiple-testing-annex.md)
- Trace: `REQ-ADVISOR-ONLINE-FDR-ANNEX-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 4 (v2 Research-Driven Credibility Tranche)
- Decision record: PRD §13 Q3 (operator answer 2026-07-27); `docs/dev-notes/research-gap-analysis-2026-07-11.md` § B7 + § C2; `research/backtesting/papers.md` [73]; pattern predecessor story `4-1-advisor-overfitting-scorecard` (ADR-0075).

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List
