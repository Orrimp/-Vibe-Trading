---
review: adversary
target: _bmad-output/planning-artifacts/architecture.md (spine, status draft, 2026-07-24)
lens: "Construct two units one level down that each obey every AD to the letter yet still build incompatibly"
reviewer: BMAD Reviewer Gate — ADVERSARY lens
date: 2026-07-24
verdict: CONDITIONAL — the ADs are individually sound and mostly enforcement-backed, but the three sanctioned seams and the migration plan each leave at least one shared entity with two (or three) writable homes; 2 critical + 3 high holes need new/tightened AD wording before the next feature or migration phase executes.
---

# Adversary Review — Architecture Spine "trading"

Method: for each attack I name two units one level down — feature teams building through
the three AD-8 seams, migration-phase executors of `spec/dev-notes/bmad-migration-plan-2026-07-24.md`
(ratified 2026-07-24), or crates evolving under AD-14 — that each **comply with every AD as
written** yet produce incompatible builds. Every claim below was verified against the repo
(file:line cited); attacks that a cited ADR already closes are tiered LOW per the gate rules.

## Findings index

| # | Tier | Hole (one line) |
|---|---|---|
| A1 | **CRITICAL** | The arm seam has THREE homes (backtest lists, ui field concat, agent forward registry); AD-8 names one — a fully compliant new arm, when crowned, kills the SUGGEST stage |
| A2 | **CRITICAL** | Migration dual-write window: spine says `spec/` authoritative until 5b, plan §9.5 freezes `spec/` writes at Phase 3 — two lifecycle records, and 5b re-founds the triad from a stale Phase-2 snapshot |
| A3 | HIGH | The ADR-corpus move is phase-orphaned: spine says Phase 4, the ratified plan's phase table never assigns the `git mv`, and the atomicity lint repoints only in 5b — two homes for new ADRs mid-migration |
| A4 | HIGH | The overlay seam has no composition law: `DrawdownControlOverlay<S>` is generic over any Strategy and its `quantity_scale` SHADOWS (does not fold) the inner overlay's scale — two AD-16-green overlays stack into a silent partial no-op |
| A5 | HIGH | Mirror shapes are dual-homed by design, but AD-14c misstates their home ("core-typed mirror structs" — they live in `ui`) and names no shape-drift gate; `cargo tree -p ui` catches edges, never fields |
| A6 | MEDIUM | Per-arm robustness seeds are POSITIONAL over a frozen 16-salt table that today's 20-candidate field already wraps — merge order of two teams' arms re-seeds each other, and AD-17 doesn't say "append-only" |
| A7 | MEDIUM | AD-18 "on conflict, the ADR wins" points the wrong way against byte-frozen gates — live example: ADR-0063 §D4 (and the code's own rustdoc) say XOR, `derive_master_seed` does `wrapping_add` |
| A8 | LOW | AD-2's migration note moves the corpus but is silent on `anchors.toml`'s own `git mv` (closed by plan §5.1 — one line should be lifted into AD-2) |
| A9 | LOW | Seam (3) "report-annex" never says WHICH report system — the `backtest` bake-off report family vs the `reports` crate's operator success reports |
| A10 | LOW | ADR-0079 is a Registry row with no file (spine notes it); the Phase-4/5b mover needs an explicit "the table is the registry, reconcile rows↔files" instruction |

---

## A1 — CRITICAL — The arm seam is a three-home entity; the spine names one home

**The seam as written (AD-8, Rule):** "features land through exactly three seams — (1) a
bake-off **arm** (`default_field()` / `default_ensemble_field()`) …"

**Repo reality — the arm set has three writable homes in three crates:**

1. The pre-registered id lists in `crates/backtest/src/bakeoff/mod.rs:562-661`
   (`default_field` / `default_ensemble_field` / `default_short_field` / `default_macro_field`) —
   the home AD-8 names.
2. The field **composition** in `crates/ui/src/leaderboard/runner.rs:55-61` — `advisor_field()`
   concatenates three of those lists by hand (`field.extend(...)` per list). A list that is not
   concatenated here does not run; concatenation ORDER also matters (see A6).
3. The forward-run engine registry in `crates/agent/src/runtime.rs:335` (`build_registry_for`) —
   a hand-maintained `match id` over literal arm ids, whose own doc says the caller
   "MUST NOT silently fall back". Ids absent from the match **error out the forward run**.

**The two compliant units.**
- *Unit 1 — arm team:* adds `default_pairs_field()` in `backtest`, concatenates it in
  `advisor_field()`, `write_report=false` (AD-3 satisfied), gate untouched (AD-1 satisfied),
  additive (AD-8 satisfied to the letter — both named functions "unchanged", extension list added
  exactly like ADR-0071/0072/0073 did). Ships green: build, anchors 119/119, all identity tests.
- *Unit 2 — SUGGEST-stage owner:* `agent`'s supervisor + `build_registry_for`, compliant with its
  own F5b contract (no silent proxy fallback), unchanged because AD-8 required nothing of it.

**Divergence artifact:** the new arm bakes, ranks, and is **crowned** on ANALYZE; the operator
clicks into SUGGEST and `build_registry_for` returns `Err` — the forward run dies for exactly the
recommendation the product just made. This is not hypothetical: the spine's own Deferred row
("R1 forward-coverage refactor") admits "`build_registry_for` lacks forward-run coverage for the
14 arms added after F5b — crowning one fails the forward run." The enforcement gap is verified:
`crates/agent/tests/forward_run_engine_fidelity.rs` asserts only the F5b-original ids
(`v0.sma`, `v0.5.macd/rsi/bbands`, `v0.buyhold`) — there is **no completeness test** that every
`advisor_field()` id resolves. There is also no duplicate-id guard across the concatenated lists
(grep of `bakeoff/mod.rs` for dedup/unique: none) — two teams picking the same literal id would
double-run an arm and collide every id-keyed KPI.

**Why this is a spine hole, not just a repo bug:** AD-8 sanctions the seam and its Rule enumerates
the *entry* functions only. A builder can obey every AD and still ship a crowned-but-unrunnable
arm. Deferred says R1 "can wait" — for a *feature-complete* product it can; for the seam the spine
holds open for future work, it cannot: the very first use of seam (1) hits it.

**Tightening (amend AD-8 Rule, seam (1)):**

> An arm lands across ALL THREE homes in one change: (a) a pre-registered id list in
> `backtest::BakeoffConfig`; (b) its concatenation into `advisor_field()`
> (`crates/ui/src/leaderboard/runner.rs`) — appended after all existing lists (AD-17); and
> (c) a resolvable engine mapping in `agent::runtime::build_registry_for`, OR an explicit
> forward-ineligible registration that suppresses the forward CTA for that crown.
> **Enforced by** a completeness test: for every id in `advisor_field()`,
> `build_registry_for` returns `Ok` or the id is in the declared forward-ineligible set; ids are
> unique across the concatenation.

And move **R1 out of Deferred into a decision**: either build the completeness test + registry
now, or register the 14 uncovered arms as forward-ineligible explicitly. A Deferred row that
means "the sanctioned seam's happy path bails at runtime" is a decision needed now.

---

## A2 — CRITICAL — Migration dual-write window: two lifecycle records, and 5b re-founds from a stale snapshot

**The conflicting texts.**
- Spine header (lines 23-24): "`spec/` remains authoritative until Phase 5b cutover."
- AD-4 migration note: "Phase 5b re-founds the same triad as story-status ↔ trace ↔ CHANGELOG;
  the invariant survives the re-homing."
- Ratified plan §8 Phase 2: generate ~120 stories + `sprint-status.yaml`; "**copy** `trace.toml`
  to new path (not yet authoritative)."
- Ratified plan §9.5: "freeze `spec/` writes at **Phase 3**; route all new writes through BMAD
  thereafter."

**The two compliant units.**
- *Unit 1 — a fix-forward committer between Phase 3 and 5b:* CI is currently red (plan §5.3:
  "CI is currently red (unrelated run-2 shakeout, task open)"), so fix commits during the
  migration window are *expected*, not hypothetical. Any fix that flips a status, adds a
  CHANGELOG line, or touches trace obeys **AD-4** ("`feature.md status:` is the lifecycle source
  of truth" — and per the spine header, `spec/` is authoritative until 5b) and writes to
  `spec/<slug>/feature.md` + `spec/trace.toml` + `CHANGELOG.md`.
- *Unit 2 — the Phase-5b executor:* obeys the plan — treats the Phase-2-generated
  stories/`sprint-status.yaml` and the Phase-2 **copy** of `trace.toml` as the content base,
  makes the copied `trace.toml` "authoritative at new path", re-founds `spec_lint`, and
  `git rm -r spec/`.

**Divergence artifact:** between Phase 2 and 5b there are **two `trace.toml` files** and two
lifecycle records with no freshness rule between them. Unit 1's post-Phase-2 changes exist only
in `spec/`; Unit 2 cuts over the stale copies and then **deletes the only current record** with
`git rm -r spec/`. The re-founded triad lint either goes red at cutover (best case) or passes
against a story set that silently lost the late change (worst case — AD-4's exact "derived
indices contradict the feature record" failure, now unfalsifiable because the feature record is
gone). Note the plan's §9.5 freeze-at-Phase-3 instruction directly contradicts the spine's
authoritative-until-5b claim — a builder can be "compliant" with either and clash with the other.

**Tightening (new AD, or an AD-4 Rule amendment; also supersedes plan §9.5's wording):**

> **Migration write-lock.** From Phase 3 until the 5b cutover commit, `spec/` remains the ONLY
> writable home for lifecycle state (feature.md status, trace.toml, CHANGELOG); every
> `_bmad-output` story/sprint-status/trace file is a read-only projection. The Phase-5b executor
> MUST regenerate stories, `sprint-status.yaml`, and `trace.toml` from `spec/` state at the
> cutover commit's parent — never reuse Phase-2 output as-is — and MUST diff the regeneration
> against the Phase-2 output; any drift is reviewed before `git rm -r spec/`.

---

## A3 — HIGH — The ADR-corpus move is phase-orphaned; two homes for new ADRs mid-migration

**The conflicting texts.**
- Spine, twice: "migration Phase 4 rebases them to
  `_bmad-output/planning-artifacts/architecture/decisions/`" (header) and "Files move to … in
  migration Phase 4, which rebases all links here" (Decision Record section).
- Ratified plan §8 **Phase 4** row: moves `dev-notes`, `runbooks`, `design`,
  `ui-design-principles.md`, the registers, and the plan file — the ADR `git mv` is **not in the
  row**. The mapping row (§4.1) lists the mv but assigns no phase; `adr_registry_check.py`'s
  repoint appears only in the **Phase 5b** machinery list.
- AD-18 Rule (literal): "every non-trivial decision is a numbered, dated, immutable ADR under
  `spec/architecture/adr/` **plus** its Registry row … written in the same commit."

**The two compliant units.**
- *Unit 1 — Phase-4 executor following the spine:* `git mv spec/architecture/adr/ →
  planning-artifacts/architecture/decisions/` in Phase 4 and rebases the spine's links. But
  `adr_registry_check.py` (path regex ×3 on `spec/architecture/adr/`) is only repointed in 5b —
  the pre-commit/CI atomicity lint that AD-18 names as its enforcement goes red, violating the
  plan's own "gates green at every commit" floor. To stay green they must pull 5b work forward —
  unplanned scope — or skip the move, falsifying the spine.
- *Unit 2 — a feature/fix team landing a decision mid-migration:* obeys AD-18's letter and writes
  ADR-0089 under `spec/architecture/adr/`. If Unit 1 already moved the corpus, the new ADR lands
  in a directory the registry README has left; if the lint was repointed, registration is
  invisible to it. Either way: **two homes for the decision record**, and the Phase-5b
  `git rm -r spec/` ambiguity ("feature folders" in the row; "spec/ is gone after Phase 5" in the
  target tree) risks deleting a not-yet-moved ADR corpus.

**Tightening (amend AD-18 Rule + fix the spine's Phase-4 claim):**

> The ADR home is, at every commit, **the directory `scripts/adr_registry_check.py` points at**
> — the lint defines the home. The ADR corpus `git mv` and the `adr_registry_check.py` repoint
> are ONE atomic commit, assigned explicitly to Phase <N> (pick 4 or 5b and make plan §8 and
> this spine agree). Until that commit, new ADRs go to `spec/architecture/adr/`; after it, to
> `planning-artifacts/architecture/decisions/`. `git rm -r spec/` in 5b MUST assert the ADR
> corpus (and anchors.toml, A8) have already moved.

---

## A4 — HIGH — The overlay seam has no composition law; two AD-16-green overlays stack into a silent partial no-op

**The seam as written (AD-8, Rule):** "(2) a strategy **overlay** (`Strategy::quantity_scale`)".
AD-16 requires each overlay to ship a day-1 baseline-equity-divergence e2e "from day 1" —
explicitly to kill the "scale computed but never applied" class (the 2026-05-22 incident).

**Repo reality:**
- `crates/strategy/src/drawdown_control_overlay.rs:163` — `DrawdownControlOverlay<S: Strategy>`
  wraps **any** inner Strategy ("wraps any inner [`Strategy`]", line 157).
- Its `quantity_scale` (line 390) returns **only its own** cached multiplier —
  `self.cached_multiplier.to_f64().unwrap_or(1.0)` — it never calls
  `self.inner.quantity_scale(symbol)`.
- `VolTargetingOverlay::quantity_scale` (`vol_targeting_overlay.rs:735`) likewise returns only
  its own `scale_cache` value; the `Strategy` trait default (`traits.rs:24`) returns 1.0.
- The engine reads ONE flat value per signal
  (`crates/backtest/src/scenarios/garch_vol_target_overlay.rs:279`).

**The two compliant units.**
- *Unit 1 — overlay team A:* ships `VolTargetingOverlay`, AD-16 e2e green (diverges ≥ 1 bp vs
  un-targeted baseline).
- *Unit 2 — overlay team B:* ships the Deferred-invited "overlay v0.2" (TIPP/ratcheting floor)
  as a wrapper in the `DrawdownControlOverlay` mold — generic over `S: Strategy`, AD-16 e2e
  green (its wrapper over a plain momentum inner diverges vs baseline).

**Divergence artifact:** the natural composition `B<VolTargetingOverlay>` — structurally legal,
type-checks, every AD satisfied — **silently discards the vol-targeting scale**: the outer
`quantity_scale` shadows the inner's, and the engine's single flat read has no way to see the
inner value. Team A's overlay becomes a no-op inside the stack — precisely the incident class
AD-16 exists to prevent, yet both AD-16 tests stay green because each tests its overlay **in
isolation against baseline**, never the stacked pair. Nothing in AD-8 or AD-16 defines (or
forbids) stacking.

**Tightening (amend AD-8 seam (2) + AD-16 Rule):**

> **Composition law:** `quantity_scale` is multiplicative-through — a wrapper overlay MUST
> return `own_scale * self.inner.quantity_scale(symbol)` (the ADR-0038 §D5 strategy-side
> composition mechanism only works if every layer folds the layer below). Until an ADR ratifies
> multi-overlay stacking, at most ONE `quantity_scale`-bearing overlay is active per engine.
> AD-16 addendum: when two or more overlays are active in one build, the divergence e2e MUST
> exercise the outermost composed stack (each layer's removal must move equity by the epsilon),
> not each overlay in isolation.

---

## A5 — HIGH — Mirror shapes are dual-homed with no shape-drift gate; AD-14c misstates where mirrors live

**The AD as written:** Design Paradigm + AD-14c: "Engine results cross into `ui` only as
`core`-typed **mirror** structs over mpsc channels (`BakeoffReportMirror`, `ForwardPlanView`, …);
`ui` never imports the engine crates … **`cargo tree -p ui` unchanged is a hard gate**."

**Repo reality:**
- The mirrors are declared **in `ui`**: `BakeoffReportMirror` at
  `crates/ui/src/leaderboard/state.rs:563`; `ForwardPlanView` at
  `crates/ui/src/forward_plan/state.rs:286`. "core-typed" is true only of their *fields*.
- `ForwardPlanView` is a hand-maintained "field-for-field parallel" of the agent-owned
  `agent::config::ForwardPlan`, converted by a `from_plan` adapter; the agent hands
  `ForwardPlan` over `forward_plan_rx` to the `cockpit_live` bin (which *does* depend on
  `agent` under the `live` feature — sanctioned by silence in the mermaid, but jarring against
  the sentence "`ui` never imports the engine crates").
- ANALYZE compute does not "cross from the engine" at all: `ui`'s own leaderboard runner invokes
  `backtest` directly over the sanctioned `ui → backtest` edge and builds the mirror in-crate.

**The two compliant units.**
- *Unit 1 — engine team* extends the forward plan: adds a field to `agent::config::ForwardPlan`
  (or, reading the spine literally — "new shared primitives are homed in `core`" — homes a new
  result struct in `core` and sends it from `agent`). No forbidden edge touched;
  `cargo tree -p ui` **unchanged** (the gate counts crates, not fields or types).
- *Unit 2 — UI team* evolves `ForwardPlanView` + `from_plan` independently, per the existing
  in-`ui` precedent.

**Divergence artifact:** two owners of one entity's shape with a hand adapter between them and
**no compiler or gate coupling**: a source-struct field addition compiles clean while the mirror
silently omits it (or a semantic change — units, annualization — maps wrong); alternatively the
two teams home the "same" new mirror in two places (`core` per the spine's sentence, `ui` per
precedent). AD-14c's hard gate is aimed at edges and cannot see any of this.

**Tightening (reword AD-14c Rule (c) + Design Paradigm):**

> Mirror structs are **`ui`-owned view types whose fields are `core`/std-typed** — they are NOT
> homed in `core` (`core` gains domain primitives, never view models). Each mirror pair
> (`agent::config::ForwardPlan` ↔ `ui::forward_plan::ForwardPlanView`;
> backtest result ↔ `BakeoffReportMirror`) has exactly ONE adapter site, and the adapter MUST
> exhaustively destructure the source struct (`let Source { a, b, c } = src;` — no `..`) so any
> source-field addition breaks the build until mirrored or explicitly discarded. Invocation
> paths, named: ANALYZE/CALIBRATE compute runs on `ui`'s runner thread via the `ui → backtest`
> edge; SUGGEST/paper/narration results cross from `agent` over the channel seams; the
> `live`-feature `ui → agent` bin edge is sanctioned.

---

## A6 — MEDIUM — Positional per-arm seeds over a 16-salt table the field has already outgrown; AD-17 omits "append-only"

**The AD as written (AD-17):** "all randomness flows from `ChaCha20Rng::from_seed` on the config
seed (**sub-seed determinism for per-path/per-arm streams**)."

**Repo reality:** `crates/backtest/src/bakeoff/bootstrap.rs:90` —
`derive_master_seed(seed, candidate_index)` = `seed.wrapping_add(SALT_TABLE[candidate_index % SALT_TABLE.len()])`,
where `SALT_TABLE` is a **frozen `[u64; 16]`** and `candidate_index` is "the 0-based insertion
index in the bake-off field". The advisor field is already **20 candidates** for BTC/ETH
(`advisor_field_arm_count`, `crates/ui/src/leaderboard/runner.rs:72`) — so candidates 16-19
**wrap onto salts 0-3 and share master seeds** with candidates 0-3, voiding the table's stated
purpose ("two candidates in the same bake-off do NOT share resample draws even if their equity
curves happen to be identical") for those pairs. ADR-0063 §D4 froze the table; no ADR addresses
growth past 16 or list-order stability.

**The two compliant units:** arm team A and arm team B (each fully A1-checklist-compliant) merge
in either order, or one inserts its `field.extend(...)` line above the other's in
`advisor_field()`. Both AD-17-compliant — everything is still deterministic *given* the merged
order.

**Divergence artifact:** each team validated its arm's robustness verdict pre-merge at index N;
post-merge the other team's position choice shifts it to N+k — a **different master seed**, a
different bootstrap distribution, and a possibly flipped Robust/Fragile flag, silently, with the
"same seed". Neither AD-1 (gate code untouched) nor AD-3 (no reports) nor any identity test sees
it. Additionally both new arms land at wrapped indices ≥ 16, inheriting salt-sharing with
early-field arms. The spine gives a builder no warning that field *composition and order* are
part of the reproducibility key.

**Tightening (amend AD-17 Rule):**

> Per-arm streams are POSITIONAL (`derive_master_seed(seed, candidate_index)` over a frozen
> 16-entry salt table — ADR-0051 D1 · ADR-0063 §D4): arm lists are **append-only, after all
> existing lists** — mid-list insertion re-seeds every later arm's robustness stream and is a
> breaking change requiring an ADR. Growing the field past 16 candidates requires an ADR
> extending `SALT_TABLE` (the current 20-candidate field already wraps: candidates 16-19 share
> salts 0-3 — flagged for the backlog).

---

## A7 — MEDIUM — "On conflict, the ADR wins" points the wrong way against byte-frozen gates (live XOR-vs-ADD example)

**The AD as written (AD-18):** "This spine compresses; it never overrides — **on conflict, the
ADR wins.**"

**Repo reality — an existing ADR↔code conflict:** ADR-0063 §D4 (lines 150-152) specifies
"`master_seed = seed_to_u64(req.seed) ^ candidate_index_salt`" (XOR) — and even the rustdoc
directly above the implementation says "The XOR with the salt ensures…" — but the code does
`bakeoff_seed_u64.wrapping_add(salt)` (`bootstrap.rs:90-93`). The binding text and the shipped
bytes disagree today.

**The two compliant units.**
- *Unit 1 — a re-implementer* (the R1 forward-coverage refactor is the top queued refactor; or
  any port/extraction of the bootstrap): follows AD-18 — the ADR is "the binding text" — and
  implements XOR. Different master seeds, different resample draws, robustness flags flip.
- *Unit 2 — everyone else*, relying on AD-1/AD-3/AD-17's byte-frozen behavior and the
  determinism tests locking in ADD.

Both are compliant with the spine as written; the builds are behaviorally incompatible, and
Unit 1 can cite the spine's own precedence rule in review.

**Tightening (append to AD-18 Rule):**

> "The ADR wins" governs intent and precedence **among documents**; it never licenses changing
> frozen behavior. Where an ADR's letter and byte-frozen enforcement (the AD-1/AD-2/AD-3/AD-17
> gates) disagree, the as-built bytes stand and the divergence is recorded in a NEW as-built
> ADR — never resolved by editing code to match the stale clause. (Standing instance:
> ADR-0063 §D4 says XOR; `derive_master_seed` ships ADD.)

---

## A8 — LOW — AD-2 is silent on `anchors.toml`'s own move (closed by the ratified plan)

AD-2's Rule cites `spec/anchors.toml` and `scripts/verify_anchors.sh`, and its migration note
covers only the **corpus** move to `evidence/`. A Phase-3 executor working from the spine alone
would leave `anchors.toml` in `spec/` (to be destroyed or duplicated later — two writable homes
for the anchor registry if Phase 3 copies rather than moves). **Closed** by plan §5.1
("The file does `git mv` to `evidence/anchors.toml` so it travels with the corpus") — hence LOW —
but AD-2 is the single most safety-critical AD and should carry the one line itself:
"`anchors.toml` travels with the corpus (`git mv` → `evidence/anchors.toml`) in the same Phase-3
commit that base-swaps `verify_anchors.sh`."

## A9 — LOW — Seam (3) "report-annex" never names WHICH report system

The workspace has two report systems: the `backtest` bake-off/anchored report family (where the
scorecard annex lives, guarded by AD-1 identity tests + AD-3) and the `reports` crate's operator
success reports (read-only over `audit`, ADR-0015). AD-8 seam (3) says only "a **report-annex**
(report-only KPI/scorecard)". Two teams could home "the same" KPI (e.g. a turnover figure) in
each system with different formulas — two owners of one metric, disagreeing on screen vs
operator report, with every AD green (AD-1 binds only ranking). One clarifying line closes it:
"annex = a section/KPI of the bake-off report family computed in `backtest` (mirrored to `ui`);
a metric surfacing in operator success reports is re-RENDERED from the same computation, never
re-derived."

## A10 — LOW — ADR-0079 row-without-file needs an explicit mover instruction

The spine already discloses "0079 exists as a registry row without a standalone file" (and the
run memlog flags it to the Phase-4 mover). Still, a mover that enumerates *files* would silently
drop 0079's record while a mover that copies the *table* keeps it — two "faithful" migrations
with different decision records. Add to the A3 tightening: "the Registry **table** is the
registry; the mover MUST reconcile rows↔files (0054 = intentional skip, 0079 = row-only) and
carry row-only entries forward."

---

## What held under attack (for balance)

- **AD-1/AD-3/AD-6** — the frozen gate + `write_report=false` + benchmark exemption resisted
  every "smuggle a gate change through a seam" construction I tried; the identity-test pattern
  plus default-is-byte-identical tests close the obvious pairs.
- **AD-5 (PIT)** — the no-public-ctor type plus the raw-join lint leaves no two-team divergence
  path I could construct at this level.
- **AD-9, AD-15, AD-19** — type-system-enforced or absence-enforced; no compliant-but-
  incompatible pair found.
- **AD-2's before-AND-after wording** — correctly forces both migration executors through the
  same 119/119 bar, which is what makes A8 merely LOW.

## Disposition requested

Close A1-A5 with the proposed AD amendments (or equivalent wording) before: any use of seam (1)
or (2) [A1, A4, A6], migration Phases 3-5b [A2, A3, A8, A10], or the R1 refactor [A1, A7].
A6/A7 additionally surface two latent repo-level items worth backlog rows independent of the
spine: the SALT_TABLE wrap at field size > 16, and the ADR-0063-vs-code XOR/ADD divergence.
