# Story 4.13: advisor-online-fdr-annex

Status: ready-for-dev

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

- [ ] Architect M-T1: ratify ledger location/format + annex math (its own ADR, AD-18 atomic with Registry row); confirm the anchored CLI report path is untouched (scorecard precedent: advisor bakeoff runs `write_report = false`; annex carried on `Recommendation`).
- [ ] Ledger writer at bakeoff completion — proposed seam: the scorecard block `crates/backtest/src/bakeoff/mod.rs:1167-1180` already has `all_sharpes`/`N_eff`/`t_bars`/crown in scope.
- [ ] Annex math module (`bakeoff/fdr_annex.rs` or sibling) + render line beside the scorecard on leaderboard/forward-plan surfaces (pixel proof per AD-10 if UI-visible).
- [ ] Identity test (AC 4) + degradation tests (AC 5) + a unit fixture proving the expected-false-positive arithmetic on a known sequence.
- [ ] Gates: anchors 119/119, spec-lint, clippy, fmt.

## Dev Notes

- **Design question (proposed, architect ratifies): where do runs record their tested-hypothesis count?** Proposal: `advisor-runs/fdr-ledger.jsonl` at repo root, git-ignored (new sibling of `/lab-runs/` + `/plan-exports/` — same ADR-0055 anchor-safety argument), serde rows with Decimal-as-string. Alternative: a sub-path under the existing `/lab-runs/`. Per-run hypothesis count = the arm family actually ranked in that run (`all_sharpes.len()` / `N_eff` both recorded so the annex can use either; the gap analysis counts RUNS as the online sequence and arms-within-run as the per-run family).
- **What the dev-note actually recommends (cited, not invented):** B7 verbatim: "online-FDR / alpha-investing (`backtesting[73]`) controls the false-'beats-hold' rate across a *sequence* of re-runs; a static Šidák/Holm on the crown set is the cheap version… Rec: build-candidate — a report-annex line (the expected false-positive count at α over N runs), NOT a gate change." `research/backtesting/papers.md` [73] adds the decaying-memory rationale (a 2021-regime discovery shouldn't spend 2026's error budget) and the alpha-wealth mechanic (a long nothing-beats-hold streak tightens the budget — the correct self-skeptical response).
- **Do-not-build register check (mandatory): PASS — and register-ENDORSED.** The register's closing § "What IS still legitimately open" names exactly this annex as the gap map's one build-candidate. Designed around **E-1**: no crown-eligibility veto, no `rank.rs` read of any annex field — the annex is informational exactly like `crown_clears_dsr`. Wiring it into eligibility later would require the full E-1 four-step bar (`docs/dev-notes/dsr-report-only-decision-2026-07-09.md`) — out of scope here and stated so.
- Era-qualified thesis unaffected: the annex measures selection pressure across runs; it neither adds alpha surface nor restates the thesis.

### References

- Trace: `REQ-ADVISOR-ONLINE-FDR-ANNEX-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 4 (v2 Research-Driven Credibility Tranche)
- Decision record: PRD §13 Q3 (operator answer 2026-07-27); `docs/dev-notes/research-gap-analysis-2026-07-11.md` § B7 + § C2; `research/backtesting/papers.md` [73]; pattern predecessor story `4-1-advisor-overfitting-scorecard` (ADR-0075).

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List
