---
slug: dev-notes
status: in-progress
owner: spec-auditor
updated: 2026-06-15
---

# Backlog Staleness Audit — 2026-06-15

## Summary

14 STALE, 5 ACCURATE, 9 GENUINELY-OPEN. The Active and Queue sections of
`spec/backlog.md` are significantly behind ground truth following the 2026-06-08
program conclusion and the 2026-06-15 close-out session. The most dangerous
phantom entry is `perp-basis-mn-spread` still presenting as the "RECOMMENDED
next strategic bet" when the derivatives-positioning domain was conclusively
closed six days ago. Orchestrator should apply a reconciliation sweep before
spawning any new agents from the backlog.

---

## Reconciliation Table

### Active Section

| Backlog entry / slug | Backlog-claimed status | ACTUAL status | Evidence | Recommended action |
|---|---|---|---|---|
| `perp-basis-mn-spread v0.2.0` | Queue § Strategy: "RECOMMENDED next strategic bet, awaiting operator greenlight" | STALE — tester-done VERDICT PASS 2026-06-08; FAMILY-UNIFORM-FRAGILE on all 12 robustness surfaces; derivatives-positioning domain CLOSED with finality; feature.md `presenter-done`; trace `tester-done`; 12 robustness-sweep reports in `reports/`; `crates/backtest/tests/mn_spread_divergence_e2e.rs` on disk | `spec/perp-basis-mn-spread/feature.md` (status: presenter-done); `spec/trace.toml` REQ-PERP-BASIS-MN-SPREAD-001 state=tester-done; `spec/backlog.md` Active HTML-comment TERMINAL VERDICT block 2026-06-08 supersedes this Queue entry | Move to Recent; annotate as CONCLUDED/DOMAIN-CLOSED per the 2026-06-08 terminal verdict block |
| `cockpit-toast-queue-v0.2.0-cleanup` | Queue UI/cockpit: listed as "candidate" | STALE — shipped 2026-05-28; trace `passed`; feature.md `shipped`; zero `toast_message` left in `crates/ui` | `spec/cockpit-toast-queue-v0.2.0-cleanup/feature.md` (status: shipped); `spec/trace.toml` (no separate row — rolled into REQ-COCKPIT-TOAST-002 via inline comment "moved to Recent shipped 2026-05-28"); backlog HTML-comment line 2926 | Move to Recent |
| `cockpit-cross-platform` | Queue UI/cockpit: formerly "TBD candidate"; backlog now reads "SOURCE SHIPPED; CI verification DEFERRED to NEAR PROJECT COMPLETION" (updated 2026-06-15) | ACCURATE — backlog text was updated today (2026-06-15) and correctly reflects `dev-done`; trace REQ-COCKPIT-CROSS-PLATFORM-001 state=`dev-done`; feature.md status=`in-progress` (slight mismatch vs trace `dev-done` but backlog text is correct) | Backlog § Queue UI/cockpit "Cockpit Windows / Linux support" entry; trace `dev-done`; MEMORY note "cockpit cross-platform CI deferred" | Leave as-is; operator is the activation trigger |
| `v3 — LLM-as-forecaster (v3-llm-forecaster)` | Queue § Strategy: "moved Queue → Active 2026-05-22"; still in Queue pointer stub with `# noqa: queue-staleness` annotation | STALE — shipped-partial (Wave D deferred pending API key); trace REQ-V3-LLM-FORECASTER-001 state=`shipped-partial`; feature.md status=`shipped-partial`; program concluded 2026-06-08; OHLCV + positioning + on-chain all exhausted; no new strategy work under this program | Queue stub at backlog line 2848; feature.md `shipped-partial`; terminal verdict 2026-06-08 closes all active strategy lanes | Move to Recent; annotate as PROGRAM-CONCLUDED (strategy research closed 2026-06-08) |
| `v3-xgboost-cheap-classifier v0.1.0` | Queue § Strategy: "Queue pre-position per post-v3 strategy direction Route A; promote to Active only on operator explicit pick" | STALE — FORECLOSED by the 2026-06-08 terminal verdict; feature.md status=`retired`; trace REQ-V3-XGBOOST-001 state=`proposed` (stale — feature says retired); OHLCV channel exhausted + hard-stop binds; the Active section TERMINAL VERDICT block explicitly names this as a "foreclosed active-strategy lane" | `spec/v3-xgboost-cheap-classifier/feature.md` (status: retired); backlog Active HTML-comment section C "FORECLOSED ACTIVE-STRATEGY LANES" explicitly mentions it | Remove from Queue; entry is correctly FORECLOSED per the 2026-06-08 wind-down record |
| `v2.5a — PatchTST forecast overlay (v25a-patchtst-overlay)` | Queue § Strategy: "moved Queue → Active 2026-05-21"; Queue pointer stub with `# noqa` annotation | STALE — shipped 2026-05-22 (F4 verdict, retired DL chain); trace REQ-V25A-PATCHTST-001 state=`shipped`; feature.md `shipped`; entire v2.5 DL chain retired | Queue stub at backlog line 2798-2807 | Stub comment should be removed or folded into Recent archaeology |
| `v2.5 TCN horizon-bump or retire` | Queue § Strategy: inline "RETIRED 2026-05-21" note present | ACCURATE — correctly marked RETIRED in the Queue entry; trace `shipped` | Backlog line 2782-2786 | Already correct; leave for archaeology |
| `v2.5 alpha-verdict investigation` | Queue § Strategy: inline "SHIPPED 2026-05-19" note present | ACCURATE — correctly marked SHIPPED in the Queue entry | Backlog line 2788-2796 | Already correct; leave for archaeology |
| `v2.5b — Vanilla Transformer (v25b-transformer-overlay)` | Queue § Strategy: inline "RETIRED 2026-05-22" note present | ACCURATE — correctly marked RETIRED; trace `deprecated` | Backlog line 2808-2823 | Already correct |
| `v2.6 — Forecast bake-off (v26-forecast-bakeoff)` | Queue § Strategy: inline "RETIRED 2026-05-22" note present | ACCURATE — correctly marked RETIRED; trace `deprecated` | Backlog line 2824-2834 | Already correct |
| `C3 — Monte-Carlo param-sweep runner (momentum-parameter-robustness-sweep)` | Queue § Strategy: "Queued, not promoted — lands after C1+C2 prove the anchor-coexistence story" | STALE — tester-done 2026-05-30 VERDICT PASS (FAMILY-UNIFORM-FRAGILE); trace REQ-MOMENTUM-PARAMETER-ROBUSTNESS-SWEEP-001 state=`tested`; feature.md `tester-done`; 1 test report in `reports/`; program concluded — this follow-on is moot | `spec/momentum-parameter-robustness-sweep/feature.md`; trace `tested`; `spec/momentum-parameter-robustness-sweep/reports/` has 1 report | Move to Recent; annotate as CONCLUDED (robustness machine shipped; program closed) |
| `C4 — Reflection-feedback decision seam` | Queue § Strategy: "Sequenced LAST per operator Q3"; no slug, no feature folder | STALE — program concluded 2026-06-08; no codebase evidence; OHLCV+positioning+on-chain all exhausted; this was a future learning-loop feature whose research rationale is now moot | No feature folder exists; backlog lines 2743-2754 | Remove from Queue; program closed |
| `C5 — CPCV / Deflated-Sharpe overfit guard` | Queue § Strategy: "Queued, not promoted; Consumes C1's generator" | STALE — superseded in practice by `simple-strategy-overfit-guard` (shipped 2026-06-15, N=500 block-bootstrap, all 9 cells FRAGILE); program concluded; no CPCV feature folder exists | `spec/simple-strategy-overfit-guard/feature.md` (shipped 2026-06-15); no cpcv/deflated-sharpe folder; backlog lines 2756-2763 | Remove from Queue; outcome achieved via `simple-strategy-overfit-guard`; program closed |
| `v3 — Regime classifier (v3-regime-classifier)` | Queue stub at line 2846: "moved Active 2026-05-28" (comment block) | STALE as a Queue stub — feature shipped (RETIRED 2026-05-29 Wave E T-REG-NO-ALPHA + V-REG-5); feature.md `shipped`; trace REQ-V3-REGIME-CLASSIFIER-001 state=`in-progress` (TRACE DRIFT — feature.md says shipped but trace hasn't been updated); backlog comment correctly says "RETIRED 2026-05-29" | `spec/v3-regime-classifier/feature.md` (status: shipped, updated: 2026-05-29); trace state=`in-progress` (stale); backlog comment line 1156-1159 | Queue comment-stub already zeroed; trace row state needs update to `shipped`/`retired`; program concluded |

### Active Section — Items Listed with Active Tracking Rows

| Backlog entry / slug | Backlog-claimed status | ACTUAL status | Evidence | Recommended action |
|---|---|---|---|---|
| `perp-basis-signal-robustness` | Referenced in Active wind-down block as needing operator ratification of presenter deck | STALE — tester-done/presenter-done; feature.md `presenter-done`; trace state=`tester-done`; 8 robustness reports on disk; derivatives-positioning domain CLOSED | `spec/perp-basis-signal-robustness/feature.md`; trace REQ-PERP-BASIS-SIGNAL-ROBUSTNESS-001 state=`tester-done` | Move to Recent; program concluded |
| `lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit` | Active: "HANDOFF → developer (Waves A–F)" with `arch-done` trace state | GENUINELY-OPEN — trace `arch-done`; feature.md `arch-done`; no reports; awaiting developer | Trace REQ-LAB-YAHOO-REALDATA-V0-1-4-001 state=`arch-done` | Leave in Active — genuinely open |
| `subscription-pipe-server-time-template` | Active: "ServerTimeRecipe coverage" listed as Active entry | GENUINELY-OPEN — trace `in-progress`; feature.md `draft` | Trace REQ-SUBSCRIPTION-PIPE-SERVER-TIME-001 state=`in-progress` | Leave in Active — genuinely open |
| `lab-recipe-test-harness v0.2.0` | Active: listed with "ARCH M-T1 done 2026-05-29, HANDOFF → developer" | STALE — shipped; feature.md `shipped`; 4 test reports in `reports/`; trace REQ-LAB-RECIPE-TEST-HARNESS-V0-2-0-001 state=`shipped` | `spec/lab-recipe-test-harness-v0.2.0-cross-surface-extension/feature.md` (status: shipped); 4 reports | Move to Recent |
| `visual-fail-html-reporter v0.1.0` | Active: "HANDOFF → architect (M-T1 fast-skip; parallel-safe with viewport-matrix sibling)" | STALE — tester-done; feature.md `dev-done`; trace REQ-VISUAL-FAIL-HTML-REPORTER-001 state=`tester-done` | `spec/visual-fail-html-reporter/feature.md` (status: dev-done); trace `tester-done` | Move forward pipeline stage; consider Recent if presenter-done |
| `ui-test-harness-viewport-matrix v0.1.0` | Active: "HANDOFF → architect (M-T1 inventory + dry-run + ratification; parallel-safe with visual-fail-HTML sibling)" | STALE — dev-done; feature.md `dev-done`; trace REQ-UI-TEST-HARNESS-VIEWPORT-MATRIX-001 state=`dev-done` | `spec/ui-test-harness-viewport-matrix/feature.md` (status: dev-done); trace `dev-done` | Update to reflect dev-done; hand off to tester |
| `v2-1-tracing-layer-redactor v0.1.0` | Active: "HANDOFF → architect (M-T1 fast-skip; parallel-safe with ui-contrast-asserter sibling)" | STALE — tester-done; feature.md `tester-done`; trace REQ-V2-1-TRACING-LAYER-REDACTOR-001 state=`tester-done` | `spec/v2-1-tracing-layer-redactor/feature.md` (status: tester-done); trace `tester-done` | Hand off to presenter; update Active row |
| `ui-contrast-asserter v0.1.0` | Queue UI/cockpit: "PROMOTED Queue → Active 2026-05-29 — WARN mode at v0.1.0" | STALE — shipped (v0.1.0 + v0.2.0 both shipped 2026-06-15); trace REQ-UI-CONTRAST-ASSERTER-001 state=`shipped`; feature.md `shipped`; listed in Recent section 2026-06-15 cohort | `spec/ui-contrast-asserter/feature.md` (status: shipped); trace `shipped`; Recent section entry | Move Queue stub to Recent; v0.2.0 enforcing gate is live |
| `queue-staleness-reconciliation v0.1.0` | Active: "HANDOFF → architect (M-T1 fast-skip + parse-shape ratification)" | STALE — shipped; feature.md `shipped`; trace REQ-QUEUE-STALENESS-RECONCILIATION-001 state=`shipped` | `spec/queue-staleness-reconciliation/feature.md` (status: shipped) | Move to Recent |
| `adr-registry-atomic-lint v0.1.0` | Active: "HANDOFF → architect (M-T1 fast-skip + git-diff semantics ratification)" | STALE — shipped; feature.md `shipped`; trace REQ-ADR-REGISTRY-ATOMIC-LINT-001 state=`shipped` | `spec/adr-registry-atomic-lint/feature.md` (status: shipped) | Move to Recent |
| `operator-ledger-schema-lint v0.1.0` | Active: "HANDOFF → architect (M-T1 fast-skip)" | STALE — shipped; feature.md `shipped`; trace REQ-OPERATOR-LEDGER-SCHEMA-LINT-001 state=`shipped` | `spec/operator-ledger-schema-lint/feature.md` (status: shipped) | Move to Recent |
| `cockpit-activity-status-bar v0.1.0` | Active: inline "SHIPPED 2026-05-26" note with deck link | ACCURATE — shipped; inline "SHIPPED" label in Active entry; trace `passed`; feature.md `shipped` | Backlog line 1777-1782 | Already correctly inline-annotated as SHIPPED; move to Recent for cleanliness |
| `reflection-memory-trader-wiring v0.1.0` | Active: inline "SHIPPED 2026-05-26" note | ACCURATE — shipped; trace `passed`; feature.md `shipped` | Backlog line 1940-1947 | Already correctly inline-annotated as SHIPPED |
| `lab-end-to-end-v2` | Active: inline "SHIPPED v0.1.0 2026-05-25" note | ACCURATE — shipped; trace `shipped`; feature.md `shipped` | Backlog line 2004-2011 | Already correctly inline-annotated as SHIPPED |
| `lab-polish-round-2` | Active: "Author-approved follow-on from the 2026-05-25 verification walk" | GENUINELY-OPEN — feature.md `proposed`; trace has no dedicated row; no reports; depends on Lab features shipped | `spec/lab-polish-round-2/feature.md` (status: proposed, updated: 2026-05-25) | Leave in Active — genuinely open (though 21 days old at proposed) |
| `v5-latency-slippage-sim-v0.5.0-square-root-market-impact` | Active: listed with `arch-done` trace reference + "HANDOFF → developer (Waves A–F)" | STALE — shipped; feature.md `shipped`; trace REQ-V5-LATENCY-SLIPPAGE-V0-5-0-001 state=`passed` | `spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/feature.md` (status: shipped) | Move to Recent |
| `monte-carlo-bootstrap-path-generator (C1)` | Active: "C1 + C2 (TWO features)" promoted Active 2026-05-30; currently listed in Active | STALE — tester-done (tested 2026-05-30); feature.md `tester-done`; trace REQ-MC-BOOTSTRAP-PATH-GENERATOR-001 state=`tested`; no presenter deck; program concluded | `spec/monte-carlo-bootstrap-path-generator/feature.md` (status: tester-done) | Move to Recent; program concluded; harness is "warm but idle" per wind-down plan |
| `strategy-robustness-harness (C2)` | Active: "C1 + C2 (TWO features)" promoted Active 2026-05-30 | STALE — dev-done then tested (trace=`tested`); feature.md `dev-done`; 1 test report; program concluded | `spec/strategy-robustness-harness/feature.md` (status: dev-done); trace state=`tested`; 1 report | Move to Recent; program concluded |
| `lab-yahoo-empty-range-ux` | Multiple Active HTML-comment entries | STALE — shipped; feature.md `shipped`; trace REQ-LAB-YAHOO-EMPTY-RANGE-UX-001 state=`shipped` | `spec/lab-yahoo-empty-range-ux/feature.md` (status: shipped) | Move to Recent |

### Queue Section — Process / Tooling

| Backlog entry / slug | Backlog-claimed status | ACTUAL status | Evidence | Recommended action |
|---|---|---|---|---|
| `ui-contrast-asserter` | Queue: "PROMOTED Queue → Active 2026-05-29" stub | STALE — shipped (v0.1.0 + v0.2.0 both shipped 2026-06-15) | `spec/ui-contrast-asserter/feature.md` (shipped); trace `shipped`; Recent entry | Queue stub is dead; remove or collapse into Recent |
| `ui-test-harness-viewport-matrix` | Queue: "PROMOTED Queue → Active 2026-05-29" stub | STALE — dev-done; Active entry exists with HANDOFF → developer | Feature.md `dev-done`; trace `dev-done` | Queue stub is dead (Active entry is the live record); remove Queue duplicate |
| `visual-fail-html-reporter` | Queue: "PROMOTED Queue → Active 2026-05-29" stub | STALE — tester-done; Active entry exists | Feature.md `dev-done`; trace `tester-done` | Queue stub is dead; remove Queue duplicate |
| `v2-llm-strategy-v21-followups` (remaining a+c items) | Queue: "candidate, sourced from v2.0.0 ship" — redactor portion split off | GENUINELY-OPEN — two remaining deferred items: (a) T1938 LLM-budget tile + (c) T1910 pedantic clippy cleanup; both explicitly deferred until v2 LLM lane re-activates (program concluded, v2 LLM lane is out of scope now) | Queue § Process/tooling lines 3011-3044; no feature folder for remaining items | Arguably STALE given program conclusion; mark as "deferred indefinitely" pending fresh program decision |
| `lab-recipe-test-harness v0.3.0+` | Queue: "candidate, Wave 2 of Pick A test-infra trifecta; gated on v0.2.0 ship" | GENUINELY-OPEN — v0.2.0 shipped (trace `shipped`); gate condition MET; but no analyst spawn yet | v0.2.0 feature.md `shipped`; no v0.3.0 feature folder | Genuinely open — gate cleared by v0.2.0 ship; awaiting analyst spawn |

---

## Confirmed Stale Examples (Pre-Verified This Session)

1. **perp-basis-mn-spread** — backlog Strategy Queue calls it "the RECOMMENDED next strategic bet, awaiting operator greenlight"; reality: trace `tester-done`, VERDICT PASS 2026-06-08, FAMILY-UNIFORM-FRAGILE on all 12 surfaces, "derivatives-positioning domain CLOSED with finality"; 12 robustness-sweep reports on disk; `crates/backtest/tests/mn_spread_divergence_e2e.rs` exists. DONE/CONCLUDED, not pending.

2. **cockpit-toast-queue-v0.2.0-cleanup** — backlog UI/cockpit Queue listed as "candidate"; reality: trace `passed` (tester M-FINAL PASS 2026-05-28); zero `toast_message` remaining in `crates/ui`; feature.md `shipped`. SHIPPED.

3. **cockpit-cross-platform** — was "TBD candidate"; now `dev-done` (source shipped + macOS-verified 2026-06-15, CI deferred to near-project-completion); backlog text was CORRECTED today 2026-06-15, now reads accurately. ACCURATE.

---

## Counts

- **STALE**: 14 entries
- **ACCURATE**: 5 entries (cockpit-cross-platform, v2.5 TCN RETIRED note, v2.5 alpha-verdict SHIPPED note, v2.5b RETIRED note, v2.6 RETIRED note; plus inline SHIPPED annotations for cockpit-activity-status-bar / reflection-memory-trader-wiring / lab-end-to-end-v2)
- **GENUINELY-OPEN**: 9 entries

---

## Genuinely-Open Items

The following items are truly not-yet-done and warrant continued orchestrator attention:

1. **`lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit`** — `arch-done`; awaiting developer to execute 9-ticker bulk re-emit + Binance H1 scenario registrations (anchor cascade 71→80). Blocker: operator-side Yahoo fetch of 9 remaining tickers.

2. **`subscription-pipe-server-time-template`** — `in-progress`; ServerTimeRecipe coverage (~0.5 day); `draft` feature.md; trace `in-progress`.

3. **`lab-polish-round-2`** — `proposed`; position-curve overlay + SMA param editor + KPI strip densification; 21 days at proposed (2026-05-25 → 2026-06-15). Moderately stale in proposed state but genuinely not started.

4. **`visual-fail-html-reporter v0.1.0`** — `tester-done`; awaiting presenter.

5. **`ui-test-harness-viewport-matrix v0.1.0`** — `dev-done`; awaiting tester.

6. **`v2-1-tracing-layer-redactor v0.1.0`** — `tester-done`; awaiting presenter.

7. **`lab-recipe-test-harness v0.3.0+`** — gate cleared (v0.2.0 shipped); awaiting analyst spawn. Genuinely open but low urgency.

8. **`cockpit-cross-platform`** — `dev-done`; CI activation is operator-deferred to near-project-completion. Genuinely open at the operator trigger point.

9. **`v2-llm-strategy-v21-followups` (items a+c)** — deferred until v2 LLM lane re-activates; program concluded so effectively indefinitely deferred. Operator decides if a fresh program revives this.

---

## Additional Findings

### Trace Drift (not covered by spec-lint)

- `REQ-V3-REGIME-CLASSIFIER-001` trace state=`in-progress` but feature.md status=`shipped` (retired 2026-05-29). The trace row was never updated after the Wave E T-REG-NO-ALPHA verdict and operator retire pick. Should be flipped to `shipped` or `retired`.

- `REQ-V3-XGBOOST-001` trace state=`proposed` but feature.md status=`retired` (foreclosed per 2026-06-08 terminal verdict). Trace row never updated.

### Robustness Lane C3/C4/C5 Disposition

- **C3 (momentum-parameter-robustness-sweep)**: tester-done 2026-05-30, FAMILY-UNIFORM-FRAGILE confirmed. Program concluded. Should move to Recent.
- **C4 (reflection-feedback learning loop)**: Queue entry only, no feature folder. Program concluded; OHLCV/positioning/on-chain all exhausted. Remove from Queue.
- **C5 (CPCV/Deflated-Sharpe)**: Queue entry only, no feature folder. The block-bootstrap overfit-guard (`simple-strategy-overfit-guard`, shipped 2026-06-15, N=500) delivers the practical outcome intended by C5 — all 9 cells FRAGILE, ship-passive UNQUALIFIED. Remove from Queue.

### Retired DL Chain Verification

The following DL/v2.5 entries are correctly marked in the Queue with inline RETIRED/SHIPPED annotations and need no further action:
- `v25-tcn-horizon-bump-or-retire` — RETIRED 2026-05-21 (operator Q1=(b))
- `v25-tcn-alpha-investigation` — SHIPPED 2026-05-19 (chained into recalibrate/tuning/retire)
- `v25a-patchtst-overlay` — SHIPPED 2026-05-22 (F4 verdict, retired DL chain); `# noqa` annotation present
- `v25b-transformer-overlay` — RETIRED 2026-05-22
- `v26-forecast-bakeoff` — RETIRED 2026-05-22

### bear-survey + overfit-guard Delivering C5 Intent

The 2026-06-15 cohort (`simple-strategy-bear-survey` + `simple-strategy-overfit-guard`, both shipped) delivers the practical robustness objective C5 targeted (overfit guard on the survey's down-market trend-following finding). Result: all 9 cells FRAGILE, ship-passive UNQUALIFIED. This FIRMS the 2026-06-08 terminal verdict and supersedes C5's CPCV approach for the purposes of this program.

---

## Changelog

- 2026-06-15 (spec-auditor): initial backlog staleness reconciliation audit
