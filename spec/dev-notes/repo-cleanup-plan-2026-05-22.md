---
slug: repo-cleanup-plan-2026-05-22
date: 2026-05-22
authors: orchestrator
status: proposed
related:
  - spec/dev-notes/audit-2026-05-22.md
  - spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md
  - spec/dev-notes/v3-vol-retirement-and-c5-promotion-2026-05-22.md
---

# Repo cleanup plan — 2026-05-22

After today's 8-feature session (3 retirements + 1 P0 fix + v3-llm-forecaster v0.1.0-PARTIAL ship), the repo has accumulated some debt that's worth surveying before it compounds. This plan grades each category, calls out the load-bearing risk, and suggests an execution sequence.

## TL;DR

The repo is in good shape — anchors clean (34/34), tests green (692+), spec-lint at 88/1 (just dropped 4 from this morning). No P0 items. Six P1 categories deserve attention; the rest is P2 backlog.

**Recommended next-session focus:** P1.1 (dead-link sweep) + P1.2 (stale dev-note archival) + P1.3 (retired-feature surface review). ~half-day total. Defers P2 entirely.

## Categories

### P0 — Nothing in this bucket.

The audit-2026-05-22 P0s and P1s were all cleared during this session (ADR-0035→0037 rename, parent status flip, trace state mismatches, product+arch retirement notes, backlog Active cleanup, horizon-bump decision-record stub, noop-fix retire chain, spec-lint VALID_STATUSES update).

### P1 — Should clean up next session

| # | Category | Effort | Risk | Suggested executor |
|---|---|---|---|---|
| P1.1 | Dead-link sweep (88 remaining) | ~1-2h | LOW | spec-auditor + Edit |
| P1.2 | Stale dev-note archival (20 → ~14) | ~30m | LOW | orchestrator inline |
| P1.3 | Retired-feature surface review (7,893 LoC retired code) | ~1-2h | MEDIUM | analyst |
| P1.4 | Backlog bucket hygiene (Recent shipped/retired accumulation) | ~30m | LOW | orchestrator inline |
| P1.5 | TODO Wave markers (2 in code; both intentional) | ~5m | LOW | document or delete |
| P1.6 | Cargo.toml dep audit (post-session additions) | ~30m | LOW | developer |

#### P1.1 — Dead-link sweep

**What:** `uv run scripts/spec_lint.py` reports 88 dead-links across 1 category. Down from this morning's 90+, but still substantial. Most are cross-refs into archived/renamed feature folders or to anchors that moved.

**Why:** dead-links are silent rot — they look fine until someone clicks; the spec becomes untrustworthy at the margin. The audit-2026-05-22 P2 long-tail sweep cleared 159 → 88; another pass with the same approach should land in the 30-60 range.

**Risk:** LOW. Changes are mechanical (path updates, anchor renames). No code touch.

**Approach:** spec-auditor or general-purpose subagent walks the lint output, fixes mechanically. Cap budget at ~2 hours.

#### P1.2 — Stale dev-note archival

**What:** `spec/dev-notes/` has 20 files. Some are recent + load-bearing (today's retirement chain, the noop-fix discovery, v25-dl retrospective). Some are older (2026-05-08 memory-anchor-relock, 2026-05-10 kronos-evaluation, 2026-05-12 orchestrator-tooling, 2026-05-13 session-handoff).

**Why:** older dev-notes describe decisions that have shipped (or been superseded); they clutter the dev-note index and increase context for any future audit. The retrospective from today already references the relevant ones explicitly, so the chain stays intact through archival.

**Risk:** LOW. Move-not-delete: `spec/dev-notes/archive/2026-Q2/<file>.md`. Cross-refs in committed feature.md/decomp.md continue to work IF we use forward-link redirects (or `git mv` and accept the link rot, which the P1.1 sweep then catches).

**Candidates for archival:**
- `audit-2026-05-18.md` (superseded by audit-2026-05-22).
- `memory-anchor-relock-completed-2026-05-08.md` (action complete; no live consumers).
- `kronos-evaluation-2026-05-10.md` (one-shot eval; conclusion folded into other dev-notes).
- `session-handoff-2026-05-13.md` (one-shot session boundary).
- `iced-014-feature-analysis-2026-05-15.md` + `iced-ecosystem-evaluation` (subsumed by vendor-lock at iced 0.14.0).
- `ui-testing-direction-2026-05-12.md` (superseded by `ui-testability-deep-dive-2026-05-15`).

**Keep:**
- All 2026-05-22 dev-notes (today's chain).
- `v25-dl-reading-list-2026-05-16.md` (still useful research).
- `lumen-accent-palette-extension-2026-05-17.md` (active design system).
- `audit-2026-05-22.md` (latest audit — load-bearing reference).

#### P1.3 — Retired-feature surface review

**What:** 4 retirements this session left ~7,893 LoC of "shipped but no longer pursued" code in `crates/`:

| Feature | Code | LoC |
|---|---|---|
| v3-vol-forecaster | `garch.rs` + `vol.rs` + `vol_verdict.rs` + `train_garch.rs` + `garch_vol_target_overlay.rs` + 3 strategy builders | ~2,750 |
| v3-llm-forecaster (v0.1.0-PARTIAL) | `llm_forecaster/*.rs` (8 files) | ~3,700 |
| v25-tcn (older retire) | `tcn.rs` + `train_tcn.rs` + various overlays | ~1,400 |

**Per the retirement contract** (documented in `v3-vol-retirement-and-c5-promotion-2026-05-22.md`):
- Code STAYS in tree. It's evidence.
- Anchors stay locked. Regression contract.
- No deletion.

**So what's the review?** Not deletion. The review is:
1. Confirm each retired surface is still **anchored** (else it's silent dead code).
2. Confirm each retired surface still **compiles** cleanly under workspace clippy.
3. Confirm each retired surface has **at least one test** that exercises it (otherwise it's eligible for `#[cfg(feature = "retired-X")]` gating in a future cleanup pass).
4. Document the retired-surface inventory in a single dev-note so future operators can audit fast.

**Why:** without a periodic review, retired code becomes "I'm afraid to touch it" — the worst kind of debt. Today's noop-fix discovery happened because the operator engaged with the retired-overlay scaling logic; that pattern only works if the retired code stays auditable.

**Risk:** MEDIUM. Each retired surface is a potential rabbit hole.

**Suggested executor:** analyst (research) + spec-update for the inventory dev-note. NOT developer (no code change in scope).

#### P1.4 — Backlog bucket hygiene

**What:** `spec/backlog.md` has 97 markers (## headers + - bullets). Recent shipped/retired accumulation isn't being moved out of Active in real time.

**Why:** the audit-2026-05-22 P2 backlog cleanup pass touched the v3-vol-forecaster line but didn't sweep the entire file. Today's 4-feature retire chain (vol + rebaseline + noop-fix v0.1.0 ship + llm v0.1.0-PARTIAL ship) deserves a clean Active → Recent migration.

**Risk:** LOW. Pure markdown reorg.

**Approach:** orchestrator inline, ~30 min. Move:
- `v3-volatility-forecaster` Active → Recent (retired).
- `v3-volatility-forecaster-rebaseline` Active → Recent (retired).
- `v3-volatility-forecaster-noop-fix` Active → Recent (shipped P0 fix).
- `v3-llm-forecaster` Active → Recent (shipped-partial).
- Verify C2 stays in Queue (no change).
- Verify Phase F etc. older ships in Recent.

#### P1.5 — TODO Wave markers

**What:** Only 2 in code:
- `crates/strategy/src/llm_forecaster/types.rs:445` — `// TODO Wave C: retrieve top-K from reflection-memory` — Wave C SHIPPED; the TODO is stale.
- `crates/strategy/src/llm_forecaster/anthropic_impl.rs:410` — `// TODO Wave C: from config` — same; stale.

**Why:** stale Wave markers in code are the worst kind of comment — they imply unfinished work that's actually done. ~5 min to either update to `// Wave C: implemented at <site>` OR delete entirely.

**Risk:** LOW. Cosmetic.

**Approach:** orchestrator inline.

#### P1.6 — Cargo.toml dep audit

**What:** Today's session added several deps:
- `cost` crate added as `strategy` direct dep (Wave E).
- `rusqlite` added to `strategy` (Wave G; for llm_verdict bin).
- `pollster` added to `strategy` (Wave A; async-to-sync bridge in `on_bar`).
- `uuid`, `tokio (rt)` added to `strategy` (Wave A).
- `wiremock` likely added to `strategy` dev-deps (Wave B).
- `sqlx` added to `strategy` dev-deps (Wave E).

**Why:** none of these should be dropped, but the audit confirms (i) each is used at least once in non-test code, (ii) feature gates are correct, (iii) versions match workspace conventions. Most likely outcome: clean, no changes.

**Risk:** LOW (audit-only).

**Approach:** developer subagent, ~30 min. Output: a short dev-note confirming each dep + flagging any redundancies.

### P2 — Future / opportunistic

| # | Category | Notes |
|---|---|---|
| P2.1 | Anchor body-SHA stale fixture cleanup | Some report dirs may have older non-anchored copies. Not load-bearing; the `verify_anchors.sh` picks the lexicographically-newest match. Cleanup is cosmetic. |
| P2.2 | Long-tail backlog Recent → Archive | After P1.4, Recent will be substantial. Mid-2026 archive split is reasonable. |
| P2.3 | ADR registry table sanity check | ADR-0033 IMMUTABLE / 0037 renumbered / 0038 retired-but-locked / 0039 LLM strawman — confirm registry is internally consistent. |
| P2.4 | `vendor/iced_tiny_skia/` re-audit | Operator-locked; CLAUDE.md documents the patch maintenance contract. Re-check the maintenance contract is still aligned with iced master, OR confirm the lock is still load-bearing. |
| P2.5 | `crates/forecast/checkpoints/anchors/` review | 9 files (1 garch + 2 patchtst + 6 tcn). All anchored; per retirement contract they stay. Confirm via verify_anchors.sh that each is still referenced. |
| P2.6 | Re-baseline visual snapshots after Phase F | The 2 new Wave F baselines are byte-identical to the existing `assistant_slot__open_stub.png` (one of them) — that's intentional R9.3. P2 audit: confirm no other Phase F snapshots silently regressed. |
| P2.7 | spec_lint.py false-positive review | Some dead-link reports may be false-positives (anchors that legitimately moved). One-time review pass would refine the linter rules. |

### Out of scope (do NOT do)

- **Delete retired-feature code.** The retirement contract documents the protocol: code stays, anchors locked, no deletion. The retrospective dev-notes are the audit trail; deletion erases the evidence chain.
- **Modify ADR-0033 § D3 (F-verdict).** IMMUTABLE per operator decision; ADR-0038 + ADR-0039 are parallel-not-extension.
- **Touch `vendor/iced_tiny_skia/`.** Operator-locked per CLAUDE.md.
- **Bump iced version.** Locked at 0.14.0 per CLAUDE.md operator constraint.
- **Add new external crate deps** without an architect ADR (CLAUDE.md gate).
- **Re-emit any of the 34 anchored body-SHAs** unless under a documented wiring-bug-fix protocol (ADR-0038 § D6.b).

## Suggested execution sequence

**Single session (~half-day, ~4 hours):**

1. **P1.4 backlog hygiene** (~30 min) — quickest cleanup; sets the table for the others. Orchestrator inline.
2. **P1.5 TODO Wave markers** (~5 min) — trivial; 2 line edits in v3-llm-forecaster code. Orchestrator inline.
3. **P1.2 dev-note archival** (~30 min) — orchestrator inline + `git mv` for the 6 candidates above. Run spec-lint after to catch any cross-ref breakage.
4. **P1.1 dead-link sweep** (~1-2h) — spec-auditor subagent. Target: 88 → ~30-50. Cap budget at 2h to avoid descent into long-tail rabbit holes.
5. **P1.6 Cargo.toml dep audit** (~30 min) — developer subagent. Verifies the session's dep additions and feature gates.
6. **P1.3 retired-feature surface review** (~1-2h) — analyst subagent. Output: inventory dev-note documenting each retired surface + anchor link + test coverage status.

After this session: P2 items get scheduled opportunistically. The spec-auditor's weekly audit pass naturally picks up new P2 items as they arise.

## What this plan does NOT propose

- Aggressive deletion / over-pruning. The repo's "code stays, anchors locked" retirement contract is a strength, not a liability — it preserves the evidence chain for retrospectives like today's noop-fix discovery.
- Restructuring the spec/ directory layout. Today's structure is working.
- Touching the CLAUDE.md or AGENT.md operator contracts.
- Pre-emptive cleanup of pending work (v3-llm-forecaster Wave D is paused, not orphaned).

## Cross-references

- `spec/dev-notes/audit-2026-05-22.md` — the weekly audit that established the current cleanup baseline.
- `spec/dev-notes/v3-vol-retirement-and-c5-promotion-2026-05-22.md` — retirement contract precedent.
- `spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md` — example of why retired surfaces stay auditable.
- `spec/v3-volatility-forecaster-noop-fix/reports/test-final-2026-05-22.md` § 14 — shipped-partial precedent.
- `CLAUDE.md` § Non-negotiables — the durable cleanup constraints.
