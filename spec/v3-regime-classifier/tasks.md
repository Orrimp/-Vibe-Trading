---
slug: v3-regime-classifier
status: in-progress
owner: developer
updated: 2026-05-28
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

### Architect rows (T-AR) — DONE 2026-05-28

- [x] **T-AR1** (2026-05-28) — Locked operator M-OD Q1-Q5 resolutions
  + resolved 3 architect load-bearing follow-ons (A, B, C below).
  Documented in `feature.md § Design`:
  - **A — dispatcher prerequisite gap** = option (i) degenerate
    `CashHoldStrategy` for Volatile/Calm regimes (suppression, NOT
    liquidation; existing positions HELD; v0.2.0 v1.5-MR follow-on
    fills seam).
  - **B — K4 lesson-card embedding determinism** = option (γ) preserve
    `Chop` as deprecated-but-K4-stable + APPEND `Volatile=3, Calm=4`.
    New classifier emits only the 4 Q1=(b) variants. Embedding vector
    grows by 2 zero-init slots; legacy fixtures byte-identical.
    Escape hatch: versioned schema EmbeddingV1/V2 if vector-length-growth
    breaks downstream byte-compare (no ADR amendment required).
  - **C — Markov-switching 4-state prior specification** =
    operator-set semantic priors per ADR-0049 § D1 table {Bull μ>0
    σ²low, Bear μ<0 σ²low, Volatile μ=0 σ²high, Calm μ=0 σ²low};
    Baum-Welch refines parameter values only, no state-label
    reassignment.

- [x] **T-AR2** (2026-05-28) — Authored
  [`spec/architecture/adr/0049-v3-regime-classifier-markov-switching-verdict-shape.md`](../architecture/adr/0049-v3-regime-classifier-markov-switching-verdict-shape.md).
  Sibling to ADR-0038 (NOT extension); ADR-0033 § D3 + ADR-0038 § D1
  STAY IMMUTABLE. Covers D1-D6: D1 Markov-switching model class + EM
  contract; D2 RegimeTag ordinal encoding (γ); D3 dispatcher +
  cash-fallback; D4 V-REG / T-REG verdict shape + joint table; D5
  anchor namespace `v3.0.0-regime` (bumped from analyst Q6 default
  `v2.7.0-regime` to match v3 sibling line); D6 K-reg-2 mitigation
  max-confidence dispatcher gate at ≥ 0.70. 235 lines (under 250 cap).

- [x] **T-AR3** (2026-05-28) — Locked 6-wave M-DEV decomposition:
  - **Wave A — Markov-switching core + forward filter** (`crates/forecast/src/markov_switching.rs` NEW; RegimeClassifier trait; D1+D6 unit tests; ~5-7d)
  - **Wave B — RegimeTag extension + K4 embedding contract** (`regime.rs` edit Volatile+Calm APPEND; `embedding.rs:120-126` extension; `regime_overlay_neutrality_4state.rs` NEW; D2 escape hatch declared; ~2-3d)
  - **Wave C — Strategy dispatcher + cash-fallback** (`regime_dispatcher.rs` NEW + `cash_hold.rs` NEW in `crates/strategy`; D3 routing table; D6 confidence gate; ~3-5d)
  - **Wave D — Audit + Trail UI surface** (`JournalEntry::RegimeTag` additive variant; Phase F Trail column; strings.rs `volatile` + `calm` adds; ~2-3d)
  - **Wave E — Backtest scenarios + anchors** (2 dispatcher scenarios + V-REG + T-REG reports; 4 new anchors under `v3.0.0-regime`; 70 → 74; ~3-5d)
  - **Wave F — e2e divergence gate + tester harness** (`regime_dispatcher_end_to_end.rs` NEW; CLAUDE.md non-negotiable R-NR.6; K6 noop-fix mitigation; ~2-3d)

- [x] **T-AR4** (2026-05-28) — K4-equivalent byte-identity guard
  specified: developer Wave B ships
  `crates/reflection/tests/regime_overlay_neutrality_4state.rs`
  (analogous to `patchtst_overlay_neutrality`) re-running ≥ 1 legacy
  3-state `embedding_determinism` fixture and asserting byte-identity
  on the Bull/Bear/Chop slots. Falsifies the K4 invariant if it
  breaks; triggers the D2 escape hatch (versioned embedding schema).

- [x] **T-AR5** (2026-05-28) — `crates/reflection/src/embedding.rs`
  byte-identity contract documented in ADR-0049 § D2 + Wave B contract
  above. Architect specs the K4 contract; developer Wave B verifies
  byte-identity at compile + test time. Escape hatch declared in-scope
  (no ADR amendment if vector-length growth necessitates schema
  versioning).

### Developer rows (T-D) — ACTIVE (architect M-T1 closed 2026-05-28)

> Wave ordering: A → B → C → D → E → F. Each wave is a separate
> developer turn; B's K4 contract is **load-bearing** (blocks C-F if
> the escape hatch trips).

#### Wave A — Markov-switching core + forward filter

- [ ] **T-D-A1** — New `crates/forecast/src/markov_switching.rs`.
  4-state regression per ADR-0049 § D1 priors; Baum-Welch EM
  refinement (Δ log-lik ≤ 1e-6, max 200 iters); forward filter
  emitting per-bar posterior `[p_Bull, p_Bear, p_Volatile, p_Calm]`.
  — _acceptance: synthetic 4-regime fixture recovers per-state
  μ_s/σ²_s within 10%._
- [ ] **T-D-A2** — `RegimeClassifier` trait + `MarkovSwitchingClassifier`
  impl. Trait seam for v0.2.0+ alternate model classes.
  — _acceptance: trait-object dispatch unit test._
- [ ] **T-D-A3** — Wave A unit tests:
  - `regime_switch_rate_under_threshold` (K2 falsifier; ≤ 20/wk).
  - `dispatcher_confidence_gate_zero_when_uncertain` (D6 falsifier).
  - `dispatcher_switches_when_confident` (D6 inverse).
  - Convergence on real-Binance 2023 hourly per-pair (K1 mitigation).
  - H3 likelihood-vs-K curve logged.

#### Wave B — RegimeTag extension + K4 embedding contract (LOAD-BEARING)

- [ ] **T-D-B1** — Extend `crates/reflection/src/regime.rs`: add
  `RegimeTag::Volatile, RegimeTag::Calm` (APPEND ONLY; ordinals 3, 4).
  Display: `"volatile"`, `"calm"`. — _acceptance: existing T1802
  test family passes byte-identical._
- [ ] **T-D-B2** — Extend `crates/reflection/src/embedding.rs:120-126`:
  add `Volatile => 3, Calm => 4` arms; embedding vector length grows
  by 2 one-hot slots. — _acceptance: legacy 3-state fixtures emit
  byte-identical Bull/Bear/Chop slots (Volatile/Calm slots zero)._
- [ ] **T-D-B3** — New
  `crates/reflection/tests/regime_overlay_neutrality_4state.rs`
  (pattern from `patchtst_overlay_neutrality`). Re-runs ≥ 1 legacy
  `embedding_determinism.rs` fixture; asserts byte-identity on the
  Bull/Bear/Chop slots. — _acceptance: K4 invariant byte-identical
  for legacy fixtures; falsifier is the test itself._
- [ ] **T-D-B4 (escape hatch — execute IFF T-D-B3 fails)** — Promote
  to versioned embedding schema: `EmbeddingV1` (3-slot legacy, pinned
  to existing fixtures) + `EmbeddingV2` (5-slot, new classifier).
  Per ADR-0049 § D2 in-scope; **NO ADR amendment required**. New
  fixture builder picks V1 or V2 by classifier type.

#### Wave C — Strategy dispatcher + cash-fallback

- [ ] **T-D-C1** — New `crates/strategy/src/cash_hold.rs`.
  `CashHoldStrategy` emits `SignalKind::Hold` for every (symbol, bar).
  — _acceptance: positions HELD, never liquidated, when dispatcher
  routes to CashHold._
- [ ] **T-D-C2** — New `crates/strategy/src/regime_dispatcher.rs`.
  Stateful adapter wrapping `MomentumStrategy` + `CashHoldStrategy`.
  Routes per D3 table: Bull/Bear → Momentum; Volatile/Calm → CashHold.
  — _acceptance: routing-table coverage test (4 regimes × 2 strategies)._
- [ ] **T-D-C3** — D6 confidence gate: dispatcher only switches when
  `max_p ≥ 0.70`. Below threshold, previous regime's strategy keeps
  running. — _acceptance: D6 falsifier tests pass (Wave A defined
  the contract; Wave C wires)._
- [ ] **T-D-C4** — Transition semantics: Bull/Bear → Volatile/Calm
  suppresses NEW signals; existing positions HELD. Reverse transition
  resumes momentum forwarding. — _acceptance: transition lifecycle
  test._

#### Wave D — Audit + Trail UI surface

- [ ] **T-D-D1** — Add `JournalEntry::RegimeTag { ts, symbol, regime,
  max_confidence }` additive variant in `crates/audit`. — _acceptance:
  serde round-trip; SQLite schema migration additive only._
- [ ] **T-D-D2** — Phase F Trail UI: regime-tag-per-bar column or
  per-symbol modal (architect default = column). Register `volatile`,
  `calm` in `crates/ui/src/strings.rs::all()`. — _acceptance:
  R-NR.4 zero-new-design-tokens; spec-lint PASS._

#### Wave E — Backtest scenarios + anchors (D5 namespace)

- [ ] **T-D-E1** — 2 scenario equity-curve runs under namespace
  `v3.0.0-regime`:
  - `top10-2023-fy-regime-dispatcher-realdata` (train window).
  - `top10-2024-fy-regime-dispatcher-realdata` (val window; Q5=(b)
    10 pairs; Q2=(c) split).
  — _acceptance: deterministic byte-output; per-bar regime tag
  + max_confidence logged._
- [ ] **T-D-E2** — New V-REG bin `crates/forecast/src/bin/regime_verdict.rs`.
  Emits `regime-verdict-bs1-realdata`. Implements V-REG priority tree
  per ADR-0049 § D4. — _acceptance: V-REG-1..V-REG-5 mutual
  exclusivity test (ADR-0038 § D1 precedent)._
- [ ] **T-D-E3** — Extend `crates/forecast/src/bin/sharpe_comparison.rs`
  with regime-dispatcher dispatch arm. Emits
  `sharpe-comparison-regime-dispatcher-bs1-realdata`. — _acceptance:
  T-REG-ALPHA-UNLOCKED / T-REG-MARGINAL / T-REG-NO-ALPHA classifier
  test._
- [ ] **T-D-E4** — Add 4 new anchors to `spec/anchors.toml` under
  namespace `v3.0.0-regime`. 70 → 74. — _acceptance: `verify_anchors.sh`
  PASS 74/74; zero existing-SHA delta._

#### Wave F — e2e divergence gate + tester harness (CLAUDE.md non-negotiable)

- [ ] **T-D-F1** — New `crates/strategy/tests/regime_dispatcher_end_to_end.rs`.
  Pattern copied from `vol_targeting_overlay_end_to_end.rs`. Asserts
  dispatcher equity ≠ un-conditional v1 momentum baseline by ≥ 1 bp
  when regime tag is non-trivial. — _acceptance: divergence ≥ 1 bp on
  the 2023+2024 fixture; K6 noop-fix precedent foreclosed._
- [ ] **T-D-F2** — Pre-flight smoke: full `cargo test --workspace` PASS;
  `bash scripts/verify_anchors.sh` PASS (74/74).

### Tester rows (T-F) — ACTIVE on developer M-DEV completion

- [ ] **T-F1** — Standard test-report.md per
  `.claude/skills/rust-test/templates/test-report.md`. R1-R5 + R-NR.1-6
  conformance gate. V-REG + T-REG joint verdict per ADR-0049 § D4
  table; route R-O1/R-O2/R-O3/R-O4 per feature.md § 4-cell verdict
  tree.
- [ ] **T-F2** — Anchor lock under namespace `v3.0.0-regime`
  (ADR-0049 § D5). 70 → 74. `bash scripts/verify_anchors.sh` PASS.
- [ ] **T-F3** — 30 v2.5-chain + 40 v0.x existing anchors verified
  byte-identical (R1 + K4 invariant). Zero existing-SHA delta.
- [ ] **T-F4** — Lesson-card embedding K4 determinism verified across
  the 4-state extension (`regime_overlay_neutrality_4state.rs` PASS;
  legacy `embedding_determinism.rs` family PASS). If the D2 escape
  hatch tripped at Wave B-4, V1/V2 schema is documented in the test
  report.
- [ ] **T-F5** — R-NR.6 e2e divergence gate
  (`regime_dispatcher_end_to_end.rs`) PASS — divergence ≥ 1 bp.
  **MANDATORY** CLAUDE.md non-negotiable; K6 noop-fix precedent
  foreclosed.
- [ ] **T-F6** — Presenter deck + operator approval (M-FINAL).

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
