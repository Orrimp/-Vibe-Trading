---
slug: v3-regime-classifier
status: draft
owner: analyst
updated: 2026-05-22
---

# Tasks — v3 regime classifier

> Analyst T-A rows complete 2026-05-22 (this pass). Architect / developer
> / tester rows are **DEFERRED placeholders** per Q-SEQ HYBRID — no work
> past M-A4 until activation gate fires (C1 ships OR operator promotes).

## Analyst rows (T-A) — DONE 2026-05-22

- [x] **T-A1** (2026-05-22) — Read predecessor materials.
  Confirmed state across:
  - `spec/dev-notes/strategy-reformulation-survey-2026-05-22.md`
    § Candidate 2 (regime classification) — verbatim seed for this
    brief; cost / EV / reuse / risk framing carried forward.
  - `spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md` —
    F4-F4-F4 evidence base; non-regression invariant (30 anchors
    byte-identical); ADR-0033 § D3 F-verdict immutability; what
    NOT to chase (same task framing as v2.5).
  - `crates/reflection/src/regime.rs` (full read) — load-bearing
    seed: `RegimeTag { Bull, Bear, Chop }` enum with `Display`
    emitting `bull|bear|chop`; `REGIME_THRESHOLD_RATIO = dec!(0.02)`
    const; `classify_regime(btc_closes, at) -> Result<RegimeTag, RegimeError>`
    pure-fn signature; no I/O, no f64.
  - `spec/product.md` (briefly) — moat = "(2) memory + (4) audit";
    cost ladder; strategy lifecycle gates.

- [x] **T-A2** (2026-05-22) — Locate all existing consumers of the
  regime tagger. Mapped (grep + read), all must stay byte-identical
  per R1:
  - `crates/reflection/src/embedding.rs:24` — embeds `RegimeTag`
    into lesson-card vectors.
  - `crates/reflection/src/store/*` — `LessonCard.entry_regime` +
    `LessonCard.exit_regime` fields; persisted on disk.
  - `crates/reflection/tests/store_smoke.rs`,
    `tests/post_mortem_generate_card.rs`,
    `tests/embedding_determinism.rs`,
    `tests/store_top_k_determinism.rs`,
    `tests/regime_classifier.rs` (T1802 — boundary case + Bull/Bear/Chop
    + determinism gate). All `RegimeTag` literal references.
  - `crates/reports/tests/memory_highlights_with_lessons.rs`,
    `tests/body_no_volatile_metadata.rs`,
    `tests/report_scenarios_with_lessons.rs`,
    `tests/fixtures/build_reflection_store_{7d,90d}.rs` — fixture
    builders for Phase F Memory/Models renderer pipeline.

  Confirmed: every reference uses the 3 existing enum variants and the
  documented function signature. **R1 (backward compatibility) is
  satisfiable by Q1 = (a) + Q7 = (a) extend-in-place defaults.**

- [x] **T-A3** (2026-05-22) — Author `feature.md` brief.
  Frontmatter (`status: draft`, `owner: analyst`, `version: 0.1.0`,
  predecessor: `spec/dev-notes/strategy-reformulation-survey-2026-05-22.md
  (Candidate 2)`, sibling_picks: C1 + C5). R1-R8 + H1-H6 + Q1-Q7 +
  K-reg-1..K-reg-6 + 8-item non-regression contract + DEFERRED
  milestone section + activation contract + cost estimate (~4-6
  weeks from activation) + sequencing dependencies on C1. Brief
  surfaces 7 operator-decide Qs with analyst-recommended defaults
  (Q1 = (a) 3-state extend; Q2 = (a) statistical HMM, with Q2 = (d)
  rule-based fallback; Q3 = (a) nowcasting; Q4 = (a) regime-
  conditional position sizing; Q5 verdict shape proposed as new
  sibling-to-ADR-0033; Q6 = `v2.7.0-regime`; Q7 = (a) extend-in-place).
  All defaults stated; none autoapproved (per operator-decide
  context "Surface defaults but don't autoapprove inside the analyst
  pass").

- [x] **T-A4** (2026-05-22) — Open `[[req]]` row in
  `spec/trace.toml`. Row id `REQ-V3-REGIME-CLASSIFIER-001`;
  state = `draft`; feature = `v3-regime-classifier`;
  predecessor = `strategy-reformulation-survey-2026-05-22 § Candidate 2`;
  arch / crates / tests / anchors all empty (architect / developer /
  tester own those columns; not back-filled).

- [x] **T-A5** (2026-05-22) — Add Queue § Strategy entry to
  `spec/backlog.md`. Entry under `## Queue` → `### Strategy`
  alongside the survey's other picks; status DEFERRED — not Active —
  per Q-SEQ hybrid. Reference: this brief + the survey + the seed
  file.

## M-OD — Operator decides (Q1-Q5)

_owner: operator. **RESOLVED 2026-05-28** after analyst M-A5 refresh._

- [x] **T-OD1** (2026-05-28) — Q1 regime taxonomy = **(b) 4-state
  Bull/Bear/Volatile/Calm** (overrode analyst default of 3-state
  for richer regime semantics). K4 lesson-card embedding determinism
  becomes architect M-T1 surface — new `RegimeTag::Volatile` +
  `RegimeTag::Calm` enum variants must APPEND (not insert) to
  preserve the byte-identity contract.
- [x] **T-OD2** (2026-05-28) — Q2 training window = **(a) 2023+2024
  real-Binance hourly OHLCV** + **(c) 2023→train / 2024→val split**
  (analyst default). Reuses `data/binance/REVISION.toml` lock SHA
  `3a8b96c4…`. Defer 2022 extension to v0.2.0.
- [x] **T-OD3** (2026-05-28) — Q3 model class = **(b) Markov-switching
  regression (Hamilton 1989)** (overrode analyst default of HMM for
  long-term composability + Q4 dispatcher fit). Per orchestrator
  analysis: Markov-switching gives interpretable per-regime {μ, σ²}
  parameters that map naturally onto Q1's 4-state semantics; forward
  filter gives the dispatcher (Q4) confidence intervals for K-reg-2
  mitigation; reuses retired GARCH MLE infra; trait-based seam for
  v0.2.0+ DL upgrades.
- [x] **T-OD4** (2026-05-28) — Q4 integration mode = **(b) Strategy-
  switching dispatcher** (overrode analyst default of overlay-style
  multiplier for more expressive integration). **LOAD-BEARING for
  architect M-T1**: the dispatcher prerequisite (no v1.5 mean-reversion
  strategy exists for Chop / Volatile regimes) becomes a HARD QUESTION
  the architect must resolve. Options surfaced for architect:
    - (i) Degenerate "hold cash / Flat" strategy for Chop + Volatile
      regimes (most conservative; minimum scope impact)
    - (ii) Build v1.5 mean-reversion sibling as part of v0.1.0 scope
      (HUGE scope blow-up — likely route back to operator)
    - (iii) Regime-conditional position sizing on v1 momentum
      (degrades back to overlay; defeats Q4=(b) choice)
  Architect-recommend (i) for v0.1.0; (ii) deferred to v0.2.0.
- [x] **T-OD5** (2026-05-28) — Q5 v0.1.0 scope = **(b) all 10 USDT
  pairs** (analyst default). Matches v1 momentum's basket so H1
  +0.10 Sharpe-delta gate is computed against the right baseline.

**Cost framing (revised post-locks)**: ~5-7 weeks (was analyst-estimate
~4-6 weeks). The Q1+Q3+Q4 overrides add ~1-2 weeks: 4-state Markov-
switching with 4 distinct {μ, σ²} fits is ~1 week of architect +
dev work over 3-state HMM; dispatcher (Q4=(b)) with degenerate-cash
fallback adds ~half-week over overlay-style multiplier.

## DEFERRED — was deferred 2026-05-22; ACTIVATED 2026-05-28

> Activation gate fired 2026-05-28: operator promoted Queue → Active
> after the v2.5 TCN re-investigation analyst-halt save. C1 + C5
> outcomes detailed in feature.md frontmatter sibling_picks.
> M-A5 light-touch refresh completed by analyst agent a78dc46ac61e304ee
> (API socket-aborted at tool 34; orchestrator inline-finished
> bookkeeping). Architect M-T1 now active.

### Architect rows (T-AR) — DEFERRED

- [ ] **T-AR1** (DEFERRED) — Lock Q1-Q7 resolutions with operator;
  document final taxonomy + classifier choice + horizon + strategy
  shape + verdict + anchor + disposition in `decomp.md`.
- [ ] **T-AR2** (DEFERRED) — Author new ADR (`ADR-0037 regime-
  classification-verdict-shape` proposed name) as sibling to
  ADR-0033, defining V-PASS / V-MARGINAL / V-FAIL leaves for the
  regime-classifier verdict tree. ADR-0033 § D3 stays immutable.
- [ ] **T-AR3** (DEFERRED) — Architect M-T1 Wave decomposition
  (analyst proposes provisional shape; architect locks):
  - **Wave A — Classifier core.** Extend `crates/reflection/src/regime.rs`
    in-place (Q7 = (a) default). Add hourly classifier per Q1 + Q2
    resolution. New `RegimeClassifier` trait. Pure-fn or pure-trait;
    no I/O.
  - **Wave B — Strategy builder.** New `with_regime_overlay` builder
    in `crates/strategy` consuming the classifier output per Q4.
    Composition shape mirrors v2.5 TCN overlay pattern.
  - **Wave C — Audit + Trail.** Add `JournalEntry { kind:
    "regime_tag", ... }` row shape in `crates/audit`; Trail UI surface
    additive column or modal.
  - **Wave D — Backtest scenarios.** Two new realdata scenarios
    (`top10-2023-fy-regime-overlay-realdata` +
    `top10-2024-fy-regime-overlay-realdata` analogues) under the
    `v2.7.0-regime` anchor pin.
  - **Wave E — Verification + reports.** Forecast-distribution-
    equivalent regime-tag report + Sharpe-comparison report (mirrors
    ADR-0033 § D3 report shape but for regime task — see T-AR2).
  - **Wave F — Anchor lock + presenter.**
- [ ] **T-AR4** (DEFERRED) — Architect identifies K4-equivalent
  byte-identity guard: a `regime_overlay_neutrality` test (analogous
  to existing `patchtst_overlay_neutrality`) that re-runs at least
  one v2.5-chain anchor and asserts byte-identity to enforce R1.
- [ ] **T-AR5** (DEFERRED) — Architect verifies no
  `crates/reflection/src/embedding.rs` byte drift; lesson-card
  embedding determinism contract holds across the regime-classifier
  extension (K-reg-4 mitigation).

### Developer rows (T-D) — DEFERRED

- [ ] **T-D1** (DEFERRED) — Wave A: extend `crates/reflection/src/regime.rs`
  with hourly classifier per Q1+Q2 lock. Add tests:
  - T-D1.a: existing T1802 family must keep passing byte-identical.
  - T-D1.b: new hourly classifier accuracy ≥ Q5.1 threshold on a
    held-out fixture set.
  - T-D1.c: regime-switch rate ≤ Q5.3 threshold per H6.
- [ ] **T-D2** (DEFERRED) — Wave B: implement `with_regime_overlay`
  strategy builder. Verify v1 momentum composition.
- [ ] **T-D3** (DEFERRED) — Wave C: audit + Trail wiring.
- [ ] **T-D4** (DEFERRED) — Wave D: backtest scenarios + Sharpe-
  comparison report.
- [ ] **T-D5** (DEFERRED) — Wave E: report binaries + ADR-0037
  conformance.

### Tester rows (T-F) — DEFERRED

- [ ] **T-F1** (DEFERRED) — Standard test-report.md per
  `.claude/skills/rust-test/templates/test-report.md`. R1-R8
  conformance gate. V-PASS / V-MARGINAL / V-FAIL verdict per
  ADR-0037 (T-AR2).
- [ ] **T-F2** (DEFERRED) — Anchor lock under `v2.7.0-regime`
  (Q6 default).
- [ ] **T-F3** (DEFERRED) — 30 v2.5-chain anchors verified
  byte-identical (R1 invariant + K4-equivalent guard from T-AR4).
- [ ] **T-F4** (DEFERRED) — Lesson-card embedding determinism
  verified across the extension (K-reg-4 mitigation; existing
  `embedding_determinism.rs` family + new test if architect
  proposes).
- [ ] **T-F5** (DEFERRED) — Presenter deck + operator approval
  (M-FINAL).

## Handoff envelope (analyst → operator-decide AFTER C1 ships)

```toml
[handoff]
from        = "analyst"
to          = "operator"
feature     = "v3-regime-classifier"
trace_refs  = ["REQ-V3-REGIME-CLASSIFIER-001"]
verdict     = "READY-FOR-OPERATOR-DECIDE-AFTER-C1-SHIPS"
priority    = "medium"   # blocked on C1 ship per Q-SEQ HYBRID; no urgency
notes       = """
Analyst pass complete for Candidate 2 (regime classification) of the
strategy-reformulation survey 2026-05-22. Full design brief authored;
architect M-T1 + developer waves DEFERRED per operator-decide Q-SEQ =
HYBRID. Load-bearing finding: crates/reflection/src/regime.rs already
ships a pure-fn 3-state BTC daily-close regime tagger (Bull/Bear/Chop,
±2% threshold) that this feature extends rather than reinvents — 7+
existing test files + lesson-card embedding + Phase F UI renderer
already depend on the byte-identity of that seed (R1 invariant locked).
Seven operator-decide Qs (Q1 taxonomy / Q2 classifier architecture /
Q3 lookback-horizon / Q4 strategy consumer / Q5 verdict shape / Q6
anchor strategy / Q7 in-place vs sibling vs new-crate) surfaced with
analyst-recommended defaults; none autoapproved. Hypothesis register
H1-H6. Cost ~4-6 weeks from activation gate. Activation gate: C1 ships
AND (operator routing = promote-C2 OR Sharpe-delta on C1 ≥ +0.10).
"""

[inputs]
spec_files = [
  "spec/dev-notes/strategy-reformulation-survey-2026-05-22.md",
  "spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md",
  "crates/reflection/src/regime.rs",
  "crates/reflection/src/embedding.rs",
  "crates/reflection/tests/regime_classifier.rs",
  "crates/reflection/tests/store_smoke.rs",
  "crates/reflection/tests/embedding_determinism.rs",
  "crates/reports/tests/memory_highlights_with_lessons.rs",
  "crates/reports/tests/fixtures/build_reflection_store_90d.rs",
  "spec/product.md",
  "spec/backlog.md",
  "spec/trace.toml",
]
brief = "none — orchestrator did not pass a brief; greenfield analyst pass per Q-SEQ HYBRID against the survey § Candidate 2"

[outputs]
spec_files = [
  "spec/v3-regime-classifier/feature.md",
  "spec/v3-regime-classifier/tasks.md",
  "spec/trace.toml",                      # new [[req]] row REQ-V3-REGIME-CLASSIFIER-001
  "spec/backlog.md",                      # new Queue § Strategy entry (DEFERRED — not Active)
]
trace_rows_opened  = ["REQ-V3-REGIME-CLASSIFIER-001"]
trace_rows_updated = []
feature_folders_created = ["spec/v3-regime-classifier/"]

[open_questions]
items = [
  "Q1 — Regime taxonomy: (a) keep 3-state Bull/Bear/Chop and extend; (b) 4-state Bull/Bear/Volatile/Calm; (c) continuous-valued regime score; (d) HMM-derived hidden states. Default = (a).",
  "Q2 — Classifier architecture: (a) statistical HMM/kernel; (b) small DL classifier ~100k params; (c) ensemble; (d) rule-based extension of existing regime.rs. Default = (a), with (d) as cheap fallback.",
  "Q3 — Lookback / horizon: (a) nowcast current regime from past N bars; (b) forecast regime over next N bars; (c) both. Default = (a) at v0.1.0.",
  "Q4 — Strategy consumer shape: (a) regime-conditional position sizing on v1 momentum; (b) regime-switching strategy (requires v1.5 mean-reversion); (c) regime-as-feature feeding other strategies; (d) all 3 as opt-in builders. Default = (a) at v0.1.0.",
  "Q5 — Verdict shape: (5.1) classifier accuracy ≥70%; (5.2) Sharpe-delta ≥ +0.10 vs regime-blind baseline; (5.3) regime stability (switch rate ≤10/week); (5.4) ground-truth label source — (a) human-pinned, (b) forward-return-based, (c) HMM-derived. Default Q5.4 = (a). Analyst proposes new ADR-0037 (sibling to immutable ADR-0033) — NOT an extension of ADR-0033.",
  "Q6 — Anchor strategy: new anchors regime-classifier-bs{1,2}-realdata + regime-overlay-momentum-bs{1,2}-realdata under (a) v2.7.0-regime or (b) v3.0.0-regime. Default = v2.7.0-regime (leave v3.0.0 for survey Candidate 7 ensemble).",
  "Q7 — Existing regime.rs disposition: (a) extend in-place; (b) sibling file regime_classifier.rs; (c) new crate. Default = (a). Lesson-card embedding determinism (K-reg-4) is the load-bearing constraint forcing (a).",
]

[assumptions]
items = [
  "Operator-decide 2026-05-22 directive: Q-PICK = {C1 + C2 + C5}; Q-BUDGET ~6-8w total; Q-SEQ = HYBRID (C1 first; C2 + C5 analyst-only until C1 verdict).",
  "Operator's +0.10 Sharpe-delta vs v1 baseline gate from v25-tcn-overlay § success criterion carries forward as the canonical alpha-unlock threshold for this feature's H2.",
  "ADR-0033 § D3 F-verdict tree is immutable; this feature's verdict shape lands as a sibling ADR (proposed ADR-0037) rather than an extension. Architect re-confirms at M-T1.",
  "The 30 v2.5-chain anchor body-SHAs stay byte-identical (locked invariant). The patchtst_overlay_neutrality K4 test gives architects a precedent for the regime-equivalent neutrality guard.",
  "crates/reflection/src/regime.rs RegimeTag enum + Display + REGIME_THRESHOLD_RATIO const + classify_regime fn signature stay byte-identical (R1 lock). The 7+ downstream test files + lesson-card embedding + Phase F UI renderer all depend on this.",
  "Realdata is 10 USDT pairs hourly OHLCV 2023+2024 — same evaluation substrate as v2.5 chain. No additional data sourcing required at v0.1.0 (Q-DATA defer to follow-on if H1 fails).",
  "Apple Silicon Metal is non-load-bearing if Q2 = (a) HMM (CPU-only); becomes load-bearing only under Q2 = (b) DL classifier.",
  "Work directly on `main` per project memory (no worktrees); orchestrator commits + pushes; this analyst pass writes spec files only — no code.",
]
```

HANDOFF → operator-decide AFTER C1 ships

Open questions: see Q1-Q7 above. Activation gate: C1 verdict landed
AND (operator routing = promote-C2 OR Sharpe-delta on C1 ≥ +0.10
auto-progression). At activation, orchestrator re-spawns analyst for
M-A5 light-touch refresh before architect M-T1.
