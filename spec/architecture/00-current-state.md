---
slug: architecture-00-current-state
status: living
owner: architect
updated: 2026-07-10
---

# Current-state architecture — read this first

A single derived snapshot of the system **as it is today**: the crate map, the
load-bearing invariants in force *now*, and the advisor journey spine. Read this
**instead of** serially reading the ~88 ADRs (`adr/0001-*.md` … `adr/0088-*.md`)
plus the section files, when you just need to know how the thing is wired and what
rules you must not break.

> **This is a DERIVED convenience view, not the decision record.** The numbered
> ADRs (`spec/architecture/adr/`) remain the single source of truth for *why* each
> decision was made and *what* its exact clauses are. **On any conflict, the ADR
> wins.** This page compresses; it never overrides. Every claim below cites the
> ADR (or section file) that governs it — follow the pointer for the binding text.
>
> **Maintenance:** update this page whenever an ADR lands that changes the crate
> map, an invariant, or the journey spine. It is intentionally short — if keeping
> it current becomes a burden, that is a signal the change deserves a section-file
> or ADR edit, not a longer rollup here. (A lightweight `updated:`-freshness lint
> that warns when the newest ADR post-dates this page's `updated:` is *recommended*
> but not built — flag it to the orchestrator if drift recurs.)

---

## Crate map (today)

The workspace is 17 members (root `Cargo.toml`, virtual workspace, edition 2024).
Full layout + naming rationale: [`00-overview.md`](00-overview.md); dependency
edges + audit migrations: [`01-data-flow.md`](01-data-flow.md).

| Crate (`crates/…`) | Package | Role (one line) |
|---|---|---|
| `core` | **`trading_core`** | Base domain types — `Symbol`/`Order`/`Position`/`Signal`, `Money<C: Currency>`, `PitSeries`, `FxRate`, `FundingRate`. Depends on **no** sibling crate (ADR-0001 naming; ADR-0058 PIT; ADR-0065 FX; ADR-0068 funding). |
| `data` | `data` | Market-data ingestion/storage/replay — Binance/Coinbase/Yahoo/Deribit fetchers, `ReplayFeed`, parquet corpora + `REVISION.toml` pins, dynamic on-demand cache, market calendar (ADR-0032/0040/0056/0061/0072/0073/0084). |
| `features` | `features` | Feature engineering + indicator library (shared math for strategies/forecast). |
| `llm` | `llm` | `LlmProvider` trait + Anthropic/OpenAI-compat/Ollama clients, prompt caching, Recording/Replay/Budgeted providers (ADR-0019). |
| `cost` | `cost` | Cost telemetry (LLM tokens/infra/data) + slippage models incl. opt-in `VolScaledSpread` (ADR-0081) + the checked-in venue lot-size/min-notional filter table (ADR-0087). |
| `risk` | `risk` | Risk engine, position sizing (`FixedFractionSizer` + `budget_cap`), kill switches (ADR-0060). |
| `strategy` | `strategy` | `Strategy` trait + every strategy impl (SMA, composed MACD/RSI/Bollinger, cross-sectional momentum, pairs, vote-ensembles, overlays, DVOL/macro arms, signal-library, directional shorts) + `vol_estimator` + `PlanDescribe` (ADR-0005/0063/0067/0071/0078/0079/0080). |
| `trader` | `trader` | Reflection-memory consumer, split out of `strategy` to enforce layering (ADR-0041); read-only `recall_decision_lessons` decision-support (ADR-0074). |
| `exec` | `exec` | Exchange clients / order routing / paper-trade fill publisher. (The matching engine itself lives in `backtest`.) |
| `backtest` | `backtest` | Matching engine + scenarios + **bake-off orchestrator** + param sweep + report rendering (body-SHA anchors) + the **robustness gate** (`classify_verdict`/`rank_candidates`) + overfitting scorecard (ADR-0030/0059/0066/0069/0075/0076). Bin: `backtest`. |
| `audit` | `audit` | Double-entry ledger — journal transactions, per-symbol position accounts, audit-tick stream, `equity_snapshots`. Imports **only** `trading_core` (ADR-0024/0052). Bin/lib: `report` lives in `reports`. |
| `ui` | `ui` | iced desktop cockpit — the advisor journey + Lab/Live/Compare/etc. Never imports `strategy`/`exec`/`forecast`/`llm` (ADR-0023/0057/0059/0060/0062). Bins: `cockpit_live`, `cockpit`, `viewer`. |
| `agent` | `agent` | Top-level orchestrator + `agent::runtime::run` shared by `cockpit_live`; **bootstraps** `strategy`/`llm`/`exec`; owns narration, forward-plan, paper-loop supervisor (ADR-0060/0062/0064). Bin: `trading`. |
| `reports` | `reports` | Operator success reports — read-only over `audit` (ADR-0015). Lib + bin `report`. |
| `reflection` | `reflection` | `LessonCard` store + 32-dim deterministic embeddings + `retrieve_top_k` + regime tagger. |
| `replay-cache` | `replay-cache` | Deterministic record/replay cache (LLM + data) for reproducible runs. |
| `forecast` | `forecast` | DL/ML forecast overlays (candle/tract) + `ForecastContext`. Home of the **retired** forecaster chain (TCN/PatchTST/GARCH) — code stays, anchors locked ([`12-forecast-overlay.md`](12-forecast-overlay.md)). |

**There is no `crates/models`.** The ML/DL work lives in `forecast` + `features`.

### Dependency-direction invariants (do not violate)

- **`core` is the base** — every other crate may depend on it; it depends on none
  of them. New shared primitives (PIT, FX, funding) are homed here to avoid cycles.
- **`audit` imports nothing from sibling crates** (only `trading_core` + third-party).
  Siblings write into the ledger by importing `audit`; `audit` never imports back →
  the reconciler invariant (Σ debits == Σ credits) is provable from `audit` alone.
  [Cross-cutting invariant #1](../architecture.md); ADR-0024.
- **`ui` (lib + every bin) never depends on `strategy`, `exec`, `forecast`, or
  `llm`.** Those types are bootstrapped in `agent`; results reach `ui` as
  `core`-typed mirrors *through* `backtest`/`agent` channels (`ui` already depends
  on `backtest`, so the bake-off/advisor result crosses the *identical* sanctioned
  seam with zero new `ui` edge). [Invariant #3](../architecture.md); ADR-0059 §D1 /
  ADR-0060 / ADR-0062. `cargo tree -p ui` unchanged is a hard gate on advisor work.
- **`trader`, not `strategy`, is the reflection consumer** — the ADR-0041 split
  keeps `strategy` free of `reflection`.
- **`cost` inlines its σ̂ EWMA** rather than depending on `strategy` — a
  `cost → strategy` edge would cycle (`strategy` dev-depends on `cost`). ADR-0081 §D1.
- **Money is `Money<C: Currency>` over `rust_decimal::Decimal`, never `f64`.**
  Exact-cent aggregation, zero tolerance. [Invariant #2](../architecture.md); ADR-0003.

---

## Load-bearing invariants in force now

Each rule below is *currently enforced* by a test, gate, or lint. Break one and a
gate goes red. The ADR pointer is the binding text.

1. **The FROZEN robustness gate is byte-frozen.** `crates/backtest/src/bakeoff/`
   `{robustness,rank}.rs` — `classify_verdict` / `verdict_bands` /
   `compute_robustness_flag` / `rank_candidates` — is not edited by feature work.
   Every credibility/analytics addition (scorecard, turnover, tail metrics,
   crown-credibility) is proven *not* to change ranking by an identity test
   (`scorecard_does_not_change_ranking`, `turnover_does_not_change_ranking`). New
   arms only ever mean "more candidates face the same bar." ADR-0059 §D4/D5 ·
   ADR-0063 §D4 · ADR-0066 · ADR-0075 · ADR-0076.
2. **Anchors 119/119, byte-identical.** Every shipped backtest report body has a
   SHA-256 in `spec/anchors.toml`; `scripts/verify_anchors.sh` must print
   `ANCHORS PASS (119 / 119)` before *and* after any change. Report bodies are
   **byte-immutable** — even a link-fix mutates the SHA. Anchors are keyed by
   scenario **NAME**, not filename (grepping a filename gives false negatives).
   ADR-0038 §D6.
3. **Anchor-safety by construction via `write_report=false`.** The advisor
   bake-off / sweep / robustness path writes **no** report body, so every new arm,
   overlay, ensemble, short, DVOL/macro probe, and opt-in exec-sim mode is
   anchor-neutral *by construction* — it cannot perturb the 119. This is why the
   whole advisor tranche shipped without a single anchor re-emission. ADR-0059 §D3
   (leaned on by essentially every advisor ADR).
4. **`feature.md status:` is the single source of truth** for lifecycle; the
   derived indices must not contradict it. Two lint rules in `scripts/spec_lint.py`
   enforce it: `feature-shipped-trace-drift` (shipped ⇒ `trace.toml state="shipped"`)
   and `feature-shipped-changelog-missing` (shipped ⇒ a `CHANGELOG.md` line, the
   P6a addition). ADR-0082 (+ 2026-07-10 P6a amendment).
5. **Point-in-time discipline is structural + linted.** As-of joins go through the
   type-level `core::pit::PitSeries` (`AsOf` has no public ctor → look-ahead is
   unrepresentable), now with an additive `publication_lag_ms` (default 0 =
   byte-identical). `scripts/check_no_raw_asof_join.sh` (wired into `rust-validate`)
   fails any raw `partition_point(t<=q)` as-of join outside `core::pit` lacking a
   `// PIT-OK:` marker. ADR-0058 · ADR-0086.
6. **Buy-and-hold is benchmark-exempt from the gate.** The benchmark is the null
   hypothesis candidates are scored *against*, not a candidate that must clear the
   bar — so it is excluded from the `AllFragile` determination and is crown-eligible
   regardless of its own flag. This surfaces the honest `BenchmarkWins`, the modal
   real-crypto outcome. `classify_verdict` stays byte-unchanged. ADR-0066.
7. **LLM narration passes the faithfulness gate or falls back.** The "why this one"
   narration is read-only over the already-decided `Recommendation`, guarded by a
   frozen two-layer check: a role-locked cached prompt + a deterministic
   `llm`-free post-check (P1 wrong-crown / P2 contradicted-outcome / P3
   fabricated-number exact-string / P4 banned predict/advise phrases). Any hit →
   templated fallback. The LLM never enters ranking. ADR-0064.
8. **Additive-only, no plugin architecture.** Feature work lands through exactly
   **three registration seams** — a bake-off **arm** (`default_field()` /
   `default_ensemble_field()`), a strategy **overlay** (`Strategy::quantity_scale`),
   and a **report-annex** (report-only KPI/scorecard). No dynamic plugin/hot-load
   surface (WASM plugin deferred indefinitely, ADR-0007). The 3-seam posture is the
   v2 architecture verdict.
9. **Money math is Decimal.** See Dependency-direction invariants above. ADR-0003.
10. **UI is verified at the rendered-PIXEL layer.** Cockpit/advisor screens are
    proven by `iced_test::Emulator::screenshot` harnesses that read the *populated*
    PNG with a negative control — not unit tests, text snapshots, or a no-panic
    boot. Baselines are **macOS-canonical** (`#![cfg(target_os="macos")]`; off-macOS
    the snapshot files compile to nothing). ADR-0057 · guide:
    [`../dev-notes/iced-ui-render-verification.md`](../dev-notes/iced-ui-render-verification.md).
11. **The do-not-build register is binding; the thesis is ERA-QUALIFIED.** The
    settled dead-ends (combination-search engine, live trading, band-loosening,
    the ready-unbuilt DSR veto E-1, …) live in
    [`../dev-notes/do-not-build-register.md`](../dev-notes/do-not-build-register.md)
    and must not be re-proposed. The ship-passive claim is scoped to the **current
    era (2023+)**: the P2 corpus re-run found real, cost-annex-robust active edges
    in the early market (2017-20) that decay to ~zero by 2023+ (gate-crowned; post
    scorecard-fix none is DSR-certified) — the efficiency-migration pattern. Never
    state the universal form. (commit `61887c8`; ADR-0084.)
12. **DSR is report-only; the crown-veto stays unbuilt.** The deflated-Sharpe
    scorecard (`crown_clears_dsr`) is *informational* — it never vetoes a crown.
    P1 co-presents it on the banner as a WARN-tier "weak evidence" state
    (ADR-0085), but the gate is unchanged. Turning DSR into a hard veto is
    do-not-build **E-1** (the "ready-unbuilt" veto bar). ADR-0075 · ADR-0085 ·
    [`../dev-notes/dsr-report-only-decision-2026-07-09.md`](../dev-notes/dsr-report-only-decision-2026-07-09.md).
13. **CI is operator-PARKED.** The 3-OS (Linux/Windows/macOS) matrix is shipped +
    macOS-verified but sits inert at `.github/workflows/ci.yml.deferred`. Do **not**
    `git mv` it to `ci.yml` (which starts GitHub Actions) without explicit operator
    direction — parked through the v3 close-out (remediation P7).

---

## The advisor journey spine — DATA → CALIBRATE → ANALYZE → SUGGEST

The product is **"The Honest Advisor"** ([`../product.md`](../product.md)): pick a
coin + budget → bake off all strategies → rank under the gate → forward plan →
paper-trade the €200. A visible orientation **stepper** band (ADR-0083) maps the
journey onto the cockpit; the highlighted stage is a pure projection
`stage_for(screen, &leaderboard_state)` (DATA + ANALYZE share `Screen::Leaderboard`,
discriminated by `PanelState::Empty` vs `Ready`). Full UI: [`06-ui-and-cockpit.md`](06-ui-and-cockpit.md).

| Stage | Screen | What happens | Crates | Governing ADRs |
|---|---|---|---|---|
| **DATA** | `Leaderboard` (`Empty`) | Pick coin + budget + window; on-demand Binance fetch for any uncovered `(coin, window)`; €→USDT conversion; a data-quality/trust surface flags thin/gappy corpora (P1-7). | `ui` → `backtest`/`data`/`agent` | ADR-0061 (dynamic fetch) · ADR-0065 (EUR-FX) · `advisor-data-quality-surface` (v2, P1-7) |
| **CALIBRATE** | `Tune` (sidebar label "Calibrate") | Gate-tied hyperparameter sweep of one family; each config scored through the SAME frozen gate so overfit configs read `Fragile`; a promotable config carries the tuned strategy into the forward run. | `ui` → `backtest` | ADR-0069 (sweep) · ADR-0070 (promotion) · ADR-0083 §D4 (label) |
| **ANALYZE** | `Leaderboard` (`Ready`) | Bake off every arm + buy-and-hold; rank by Sharpe under the robustness gate; overfitting scorecard + turnover/coherent-tail (CVaR) KPIs; **crown-credibility** state on the banner; optional LLM "why this one". | `backtest` (compute) → `ui` (mirror); `agent` (narration) | ADR-0059 (bake-off/rank) · ADR-0063/0066 (gate + benchmark exemption) · ADR-0075/0076 (scorecard + tail) · **ADR-0085 (P1 crown-credibility)** · ADR-0064 (narration) |
| **SUGGEST** | `ForwardPlan` | Rule-driven forward stance + budget-aware €200 sizing; forward paper-trade the selection (P/L = equity − budget); short surfaces when a short arm is crowned; **opt-in lot-realism**; **hand-off export** of the plan. | `agent` (supervisor/plan) → `ui`; `backtest` (paper) | ADR-0062 (forward plan) · ADR-0060 (sizing + forward-paper) · ADR-0068 (shorts) · **ADR-0087 (P4 lot-realism)** · **ADR-0088 (P5 hand-off export)** |

### Crown-credibility states (P1, ADR-0085)

The recommendation banner co-presents the overfitting verdict inline for an active
crown, as a pure `crown_credibility(outcome, Option<&ScorecardView>)` projection:

- **`Passes`** — `ActiveWins` and clears DSR → a muted ✓ line.
- **`WeakEvidence`** — `ActiveWins` but *fails* DSR (the ~1/5-seed noise-crown the
  gate can produce) → an unmissable ⚠ WARN band, "did not survive the overfitting
  check — treat as weak evidence," qualifying (not negating) the still-true headline.
- **`NotApplicable`** — `BenchmarkWins`/`AllFragile` carry no badge (buy-and-hold is
  gate-exempt; the DSR is on the max-Sharpe active *loser*, so a badge would mislead).

### Mid-flight status (be precise)

- **P4 — `advisor-lot-realism`** (ADR-0087, accepted): min-notional + lot-size
  rounding as an **opt-in** exec-sim mode, default byte-unchanged. Feature is
  **`dev-done`** (built, pre-tester as of 2026-07-10).
- **P5 — `advisor-handoff-export`** (ADR-0088, accepted): a deterministic, offline,
  LLM-free serialiser of the crowned plan to a markdown checklist (NO order
  placement, NO venue API). Feature is **`arch-done`** and the **developer lane is
  running now** (a sibling developer is building `crates/ui/src/export/plan_export.rs`
  in parallel with this rollup) — treat it as **dev-in-flight**, not shipped.

---

## How to find the history

This page is a snapshot. When you need the *why* or the *full lineage*:

- **ADR registry** — [`adr/README.md`](adr/README.md) § Registry: the canonical
  one-row-per-ADR table (0001-0088, `0054` intentionally skipped), each with its
  D-clause summary, status, and date. Cited by `spec/trace.toml` `arch=` fields.
- **Section files** — [`../architecture.md`](../architecture.md) § Section file
  registry links the 13 domain sections (`00-*` … `12-*`) for deeper design detail.
- **What shipped** — [`../../CHANGELOG.md`](../../CHANGELOG.md): one line per
  implemented feature, grouped by subsystem/version (the canonical "what's-been-built"
  index; completed `feature.md` are one-line stubs pointing here).
- **Dev-notes index** — [`../dev-notes/README.md`](../dev-notes/README.md):
  categorized index of standing decisions, audits, analyses, and how-tos.
- **Per-feature narrative** — `git log -- spec/<slug>/` for any feature's history.

## Changelog
- 2026-07-10 (architect): created as the remediation-plan **P6c** current-state
  rollup — one read superseding the serial ~88-ADR + 296 KB `architecture.md`
  traversal. Derived view; ADRs remain authoritative. Grounded in root
  `Cargo.toml` (crate map), `architecture.md` cross-cutting invariants, and the
  `adr/README.md` registry (0001-0088). Cross-linked from `architecture.md` header
  + `README.md` agent-files table.
