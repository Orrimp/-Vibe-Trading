# Story 3.15: advisor-options-impliedvol-probe

Status: review

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the Deribit DVOL implied-vol regime probe (v0.dvol_regime, locked W=30, PIT-joined) - FRAGILE on BTC+ETH, the pre-registered null,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `tester-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the Deribit DVOL implied-vol regime probe (v0.dvol_regime, locked W=30, PIT-joined) - FRAGILE on BTC+ETH, the pre-registered null.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

### Review Findings

<!-- bmad-code-review 2026-08-12 (burn-down 13 of 14; commit fa56241, 4,124-line diff; run through the bmad-code-review workflow: step-01 gather-context -> step-02 three parallel layers -> triage. Layers: Blind 20, Edge 22, Auditor 10 raw — 52 deduped to 24).
     VERDICT: **FAIL**. The seam was built and the join is causal, but the executed strategy is not the pre-registered strategy, three gates are provably vacuous, and the published headline number is arithmetically the signature of the defect rather than a measurement of the signal.
     All findings anchor-impacting: NO (the arm writes no anchored body — `write_report=false`, `report_path = None`, `anchors = []`, zero DVOL rows in anchors.toml). Nothing routes to 1-25 as an anchor re-lock; only the √8575 rider as an inventory row. -->

**VERDICT: FAIL.** Three layers converged independently. What was genuinely delivered: the exogenous-series seam, a causal as-of join, and a real fetcher with a pinned corpus. What does not stand: the conclusion.

- [x] [Review][CRITICAL — all three layers] **The executed strategy is not the pre-registered strategy: warm-up holds CASH, not the coin.** `DvolRegimeStrategy::new` sets `weight: 1` — commented *"HOLD = benchmark behavior, ADR-0072 D3 M-T1.4"* — **and** `is_long: false`. Signals emit only on weight *transitions*, and warm-up computes `new_weight = 1`, so the tuple is `(1, 1, false)` → `Hold` forever. Orchestrator-verified at source. The declared state says hold the coin; the position state says flat; nothing reconciles them. **The arm cannot enter until after a post-warm-up stress episode ends**, and in a persistently-calm window it never trades at all. Contradicts ADR-0072 D3 verbatim, plus four in-code comments. The deviation biases the result **toward** the pre-declared expected null — which is precisely what pre-registration exists to prevent.
- [x] [Review][CRITICAL] **The published headline number is the signature of that defect, not a measurement.** The commit and CHANGELOG advertise *"the arm diverges from buy-and-hold ~48k/49k USDT on BTC/ETH"*. Buy-and-hold on BTC H1-2024 is ≈ +47.78% → 147,780 from 100,000; an arm sitting in cash is exactly 100,000; the gap is ≈47,780. The frozen report's published divergence is **48,082** — within 0.6% of the fully-inert value. The wiring *is* real (15 trades, Sharpe −0.190 ≠ the degraded `trades=0, sharpe=0.0000` signature), but that is established by the **trade count**, not by the assertion shipped as the proof.
- [x] [Review][CRITICAL] **Three gates are vacuous, including the mandated AD-16 one.** All compare a **10%-invested** arm (`FixedFractionSizer(0.10)`) against a **100%-invested** benchmark, so divergence is structural. (i) `dvol_regime_diverges_from_buyhold_by_at_least_1bp` passes under its own documented FAIL-before trigger (Sell branch removed) by ~1,800×, and its `dvol_equity > buyhold_equity` sanity assert passes too. (ii)+(iii) both `dvol_bakeoff_path_gate` tests were written specifically to catch the `None`-stub no-op and pass under it by 4,778×. A non-vacuous discriminator already exists in-repo and costs one line: assert `trade_count > 0`.
- [x] [Review][HIGH → bug-log #78] **A failed or stale DVOL load leaves the arm IN the ranked field as a mislabelled 100%-cash stub.** The arm is dropped only for unsupported *coins*, never on load failure; `unwrap_or_default()` yields an empty series. The parquets are **gitignored**, so this is the default state of every fresh clone and CI box, while the leaderboard renders *"Implied-vol regime (hold when DVOL < 30-day median)"*. Five code comments call the fallback a *"buy-and-hold proxy"* — false, and false **because of** the warm-up defect above. **Verified LIVE on this machine 2026-08-12**: the advisor's lookbacks end at `NOW` while the corpus is frozen at 2026-07-09, so TwoWeeks/OneMonth return **zero rows** and ThreeMonths+ freezes the median for ~34 days — with the SHA pin reporting everything healthy. Not a crown risk (verified: a flat curve fails the FRAGILE band, so `is_eligible` is false); the harm is presentational, on an honesty-first product. **Propagated by citation** to story 3-16's macro arm.
- [x] [Review][HIGH] **The DVOL loader has no coverage floor — the third sibling missing one** (basis got one 2026-08-11, funding 2026-08-12). A corpus that lost 95% of its rows loads clean; the only downstream check is `is_empty()`, which a 1-row corpus passes. Expected count is trivially derivable (daily cadence = span days), and the error shape is copy-ready from `basis_data.rs`.
- [x] [Review][HIGH] **NULL parquet cells coalesce to 0, including the value column** — the exact defect `basis_data.rs` fixed in review 1-20, re-introduced and made worse: basis's value column was a string so a NULL failed the parse, but here a NULL `dvol_close` becomes `0.0` → a **plausible** DVOL below every median → the day is scored calm and the median ring is dragged down. A NULL timestamp becomes epoch-0 and the row silently vanishes past the span filter.
- [x] [Review][HIGH] **The fetcher assumes ascending candle order in two places and checks it in neither.** `aggregate_to_daily` takes `first()`/`last()` in *insertion* order, and `dvol_close = last.close` is the single field the signal consumes. If Deribit ever returns a page newest-first, `dvol_close` becomes the 00:00 candle's close instead of the 12:00 one — a systematic 12h look-back **baked into the corpus and SHA-pinned forever** — and the cursor fallback would break after page 1, silently truncating the year. No test asserts ordering or an expected day count.
- [x] [Review][HIGH] **Warm-up burn-in is charged to the evaluation window.** The load span is exactly the backtest range with no lookback, so the 30-day ring fills *inside* the window — ~16% of H1-2024 is structurally pre-signal (compounding the warm-up defect) while the 2021-2026 history sits unused on disk. The repo's own macro loader pre-extends its span by ~99 days for exactly this reason.
- [x] [Review][HIGH] **The conclusion does not follow from the evidence.** Stated: *"FRAGILE on BOTH → the implied-vol regime signal does not beat holding."* Three undisclosed confounds: (i) the FRAGILE flag carries **zero discriminating information** — the report's own §5 says *all 19 candidates are FRAGILE*, including the crowned `v0.buyhold`; (ii) the comparison is 10%-exposed vs 100%-exposed, so the arm's ceiling on a +47.78% window is ~+4.8% and "does not beat holding" is arithmetically forced by the harness — confirmed by the sibling `v0.donchian_floor` at +4.42% on **one** trade; (iii) the arm was flat through warm-up and could not enter until a stress→calm flip. Sample: one 6-month window, two ~0.9-correlated coins ≈ one independent experiment, and the registered diagnostic kill-criterion (`dvol_diag.rs` rank-IC + cross-year sign-persistence) **was never built**.
- [x] [Review][MEDIUM] **Pre-registration was genuine — and that makes the deviation the finding.** Unlike the sibling story where a kill-criterion fired on a wiring artifact, ADR-0072 is dated and committed *before* the build, with W, cut, tie, warm-up and expected-null all locked in advance. The defect is that the **implementation departed from the registration** and the one diagnostic that could have caught it was dropped, leaving a gate that flagged 19/19 as the sole criterion.
- [x] [Review][MEDIUM] **Publication-lag probe — substantially GROUNDED, unlike the Basis row.** Traced fetcher → loader → join: the stored value is a genuine end-of-day close, and no bar can exploit the residual, so ADR-0086's DVOL row is **not** the Basis-row failure. One qualification: the key is stamped `day_open + 86_400_000 − 1`, 1 ms *before* the closing candle's value is determined, so the declared lag is understated by 1 ms and the ADR's "the key already encodes availability" is literally inaccurate. No equity impact. The loader also reads `day_close_ts_ms` while **ignoring** `day_open_ts_ms` rather than asserting the keying invariant — the same present-and-ignored pattern that made the Basis row wrong.
- [x] [Review][MEDIUM] Also found: every test touching the real corpus, the loader, or `resolve_dvol_override` is `#[ignore]`d and **no CI job runs `--ignored`**, so the loader→join→orchestrator chain has zero automated coverage; the BTC/ETH allowlist is encoded in three independent places while the `dvol_supported()` the docs reference **does not exist**; `Box::leak` runs on every resolve call (unbounded in a long-lived cockpit); the ADR-required "available for BTC/ETH only" leaderboard copy was never built, and the arm-count note says 20 unconditionally while a non-BTC/ETH coin runs 19.
- [x] [Review][Record — supersession, mirror form] **The causality proof lives downstream and is uncited here.** ADR-0086/story 3-17 audited this exact join, declared it already as-of-correct, retrofitted the explicit-lag path and added the byte-identity test `dvol_byte_identical_legacy_vs_with_lag_zero` — cited in **3-17's** trace row and appearing **zero** times in 3-15's. Likewise `p2_verdict_rerun.rs` re-ran this arm across the S3/S4/S5 corpora — the cross-era evidence this story's conclusion lacks — and published no per-arm DVOL row, so the data to qualify the claim **was generated and discarded**. Backfill both into this row.
- [x] [Review][Route→1-25, inventory row only] The √8575 annualization rider is consumed by every Sharpe in this story's evidence (BTC −0.190, ETH +0.397), ~1.06% understated and ranking-invariant. **No anchored rows are added** — the report is an unanchored test report; correct at a future re-emission, never edit the frozen body.
- [x] [Review][Found here, owned elsewhere → bug-log #79] **€200 lot realism is inert on the advisor path** — `venue_filter` is configured into every bake-off arm and never reaches the engine; `with_venue_filter_mode` has zero production call sites; the file named "ADVISOR-PATH GATE" asserts a constructor value and never calls production. Affects ~14 arms. Not this story's defect; disclosed and fixed separately.

Probes CLEAR: **chain** — this arm writes **no anchored body** (`write_report=false`, `report_path = None`, `anchors = []`, zero DVOL rows in `anchors.toml`), and the standing bakeoff exemption is confirmed (the FRAGILE flags come from `bootstrap.rs`, which resamples log-returns and never re-executes fills). **#67** clear (single-symbol arm — every order in a batch carries the stepped bar's symbol); **#69** N/A (`portfolio_exposure_cap: None`); **#71** present in the path but not binding at 10% sizing against a 0.40 cap (latent, recorded); **#72/#73** clear (no funding accrual on this path). **AD-9** clear — `Decimal` end-to-end at the seam; the fetcher's `f64` is confined to a dimensionless volatility index and never crosses into money. **Seed-collision** clear (no seed arithmetic added). **Identity-forge** clear (the arm can never emit a body, so no scenario name is forgeable). **Loop-scope** clear for the shipped path (single-symbol, cursor advances once per instant), with the latent multi-symbol hazard recorded. **Channel** clear at the strategy layer (tests use the production constructor and runner; `dvol_override` has exactly one reader and no dual-purpose field) — the gap is the untested loader→join segment.

- [ ] `advisor-options-impliedvol-probe` 0.2.0 - the base feature (tester-done)

## Dev Notes

- Source feature folder: `spec/v1/advisor-options-impliedvol-probe/` - frontmatter status **`tester-done`** (verbatim), version `0.2.0`, updated `2026-06-27`.
- Status mapping: `tester-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Advisor — `**advisor-options-impliedvol-probe**`.
- Provenance: `git log -- spec/v1/advisor-options-impliedvol-probe` (full narrative); reports under `evidence/v1/advisor-options-impliedvol-probe/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-ADVISOR-OPTIONS-IMPLIEDVOL-PROBE-001` (state=`tested`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP (F1-F9 + EUR-FX + dynamic data + PIT discipline))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
