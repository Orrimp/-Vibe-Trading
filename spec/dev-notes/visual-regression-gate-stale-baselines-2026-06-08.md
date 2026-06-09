# Visual-regression gate repair — root cause + durable fix (2026-06-08)

**Author:** ui-designer (opus)
**Trigger:** The cockpit PNG-diff snapshot suites — `render_snapshots` (8)
and `visual_snapshots` (48), 56 strict-byte fixtures total — were RED on
this machine. Operator greenlit the repair.
**Scope:** `crates/ui/tests/visual-baselines/**` (56 baseline PNGs
regenerated). **No source, no test-harness, no `theme.rs` change.** The
strict-byte determinism contract is **unchanged**.

---

## TL;DR

- **Root cause: stale baselines, NOT float drift.** All 56 failures are a
  single, real, already-shipped, already-operator-reviewed UI change: the
  **"Baseline" sidebar nav item** added by `cockpit-baseline-panel`
  (`f1c1bf3` / `580af5f`) *after* the baselines were frozen at `ec79ac0`
  (2026-05-29). The sidebar nav list shifted down one row to seat
  "Baseline" in the `work` group (`[Lab, Live, Compare, Baseline]`,
  `theme::layout::SIDEBAR_GROUPS_PHASE_C`). Nothing else changed.
- **The leading hypothesis (opt-level-3 → FMA/float reorder → subpixel-AA
  drift) is REFUTED by direct experiment.** opt-0 and opt-3 renders are
  **byte-identical**. The tiny-skia CPU-determinism premise (bootstrap H1)
  **holds**.
- **Durable fix chosen: regenerate the 56 baselines at the current opt-3
  dev default; KEEP strict-byte comparison.** The gate just *correctly
  caught* a real structural change — that is the gate working as designed,
  not a reason to weaken it.
- **No architect escalation needed.** I did not touch the determinism
  contract; I honored it. (A perceptual-tolerance proposal for *future*
  hardening is filed below as an architect decision item — but it is
  explicitly **not triggered** by this incident.)
- **Proof the gate still catches a real regression:** a deliberate
  **1-LSB** accent-color change (`0x6F`→`0x70`, one channel, 1/255) turned
  **all 56 fixtures RED** (max-delta-1 on 1628 content-area pixels in the
  smallest case). Reverted; gate green again.

---

## 1. Evidence — root cause

### 1a. The diff is localized to the sidebar, with large deltas

A throwaway per-pixel analyzer (`image` crate, RGBA `abs_diff` per channel)
on every failing baseline-vs-actual pair found, at the `floor` slot
(1280×720), an **identical** diff for *every* fixture:

```
differing : 3425 px (0.37%)
bbox      : x[0..178] y[131..401]   ← left sidebar column, upper-middle
max delta : 164   mean ~54   (1264 px at delta ≥ 65)
```

Two tells that this is **structural, not AA noise**:

1. **Large deltas.** Subpixel-AA / float-precision drift produces
   max-channel deltas of ~1–4. Here max=164, mean≈54 — that is *new ink*
   (a glyph row appearing/shifting), not anti-aliasing jitter.
2. **Identical across unrelated fixtures.** `chart_screen` and
   `strategies_ready` (entirely different content) produced the
   *byte-identical* 3425-pixel diff. That only happens if the change is in
   **shared chrome (the sidebar)**, not screen content.

### 1b. The change is the "Baseline" nav item (visual confirmation)

Cropping the diff bbox (baseline vs actual) shows:

- **Baseline PNG sidebar:** Strategies · Memory · Models · Trail · … Settings
- **Actual render sidebar:** **Baseline** · Strategies · Memory · Models ·
  Trail · … Settings   ← extra top nav row, list shifted down

This matches the shipped code exactly:
`crates/ui/src/widgets/sidebar_nav.rs:41` maps
`Screen::Baseline => BASELINE_SIDEBAR_LABEL` ("Baseline"), and
`crates/ui/src/theme.rs:773` `SIDEBAR_GROUPS_PHASE_C` work-group =
`[Lab, Live, Compare, Baseline]` with the in-code comment
"cockpit-baseline-panel v0.1.0 (R6) inserts `Baseline` after `Compare`".

### 1c. Content is byte-identical with the sidebar excluded

Re-running the analyzer with the sidebar column masked out
(`x[0..200]` at 1×, `x[0..400]` at the 2× operator slot):

```
checked = 56 baselines
content-area diffs OUTSIDE the sidebar = 0
```

Every chart canvas, strategies table, memory card, models list, compare
grid, trail panel, and assistant slot is **bit-for-bit identical** between
the opt-0 baseline and the opt-3 actual.

### 1d. Provenance — the baselines never got regenerated

`git log -- crates/ui/tests/visual-baselines/` shows the last touch was
`ec79ac0` (viewport-matrix, 2026-05-29). The Baseline screen + sidebar
entry landed afterward (`f1c1bf3`/`580af5f`), and
`git merge-base --is-ancestor ec79ac0 e7d6940` confirms the baselines
predate even the perf fix. The `cockpit-baseline-panel` ship simply missed
regenerating the full-cockpit snapshot baselines.

---

## 2. The opt-level hypothesis — REFUTED

**Hypothesis under test:** the perf fix `e7d6940`
(`[profile.dev.package."*"] opt-level = 3`) rebuilt the rasterizers
(tiny-skia, cosmic-text, vendored iced_tiny_skia) at opt-3, where the
baselines were captured at opt-0; opt-3 FMA/float-reordering would change
subpixel AA → strict-byte diffs.

**Controlled experiment.** Re-ran both suites in an isolated target dir
with the rasterizer crates forced back to opt-0:

```
CARGO_TARGET_DIR=/tmp/visreg/target-opt0 cargo test -p ui \
  --test visual_snapshots --test render_snapshots --no-fail-fast \
  --config 'profile.dev.package.tiny-skia.opt-level=0' \
  --config 'profile.dev.package."iced_tiny_skia".opt-level=0' \
  --config 'profile.dev.package.cosmic-text.opt-level=0' \
  --config 'profile.dev.package."iced_graphics".opt-level=0'
```

(Build log confirms `Compiling tiny-skia v0.11.4`,
`Compiling iced_tiny_skia v0.14.0`, `Compiling cosmic-text v0.15.0` — the
rasterizers *did* rebuild at opt-0.)

**Results:**

- **Same 8 + 48 failures**, same sidebar bboxes, as the opt-3 run.
- **opt-0 actual vs opt-3 actual, full frame, no mask → 0 differing
  pixels (byte-identical).**
- **opt-0 actual vs committed (opt-0) baseline, sidebar masked → 0 content
  diff** — identical to the opt-3 result.

**Conclusion:** opt-level has **zero** effect on tiny-skia rasterization
output on this machine (Apple Silicon). The renders are byte-stable across
opt-0 ↔ opt-3. The drift class the hypothesis predicted **does not exist
here**. The perf fix `e7d6940` is exonerated. (The earlier "chart-cache
work found them red" timing is explained by §1d: the baselines went stale
when `cockpit-baseline-panel` shipped, independent of the perf fix; the
perf-fix's own "519 integration pass" did not re-render these full-cockpit
snapshots against fresh baselines.)

---

## 3. Diff classification

| Class | Count | Verdict |
|-------|-------|---------|
| Benign float/AA drift | **0** | — (none found; determinism holds) |
| Real structural change (sidebar "Baseline" nav item) | **56** | Stale baseline of an intended, shipped, reviewed change → **safe to regenerate** |
| Real regression (bug in producing widget) | **0** | **None found.** Content byte-identical. |

No regression was mixed into the drift. The single structural change is the
operator-reviewed Baseline nav entry, which is the *correct* current UI.

---

## 4. Durable fix chosen + why

**Chosen: option 2 — regenerate the 56 baselines at the current opt-3 dev
default — combined with KEEPING strict-byte comparison.**

Rationale (why this is the durable choice *for this incident*):

1. **The incident is not float drift.** The brief's recommended option 1
   (perceptual tolerance) and option 3 (pin the rasterizer opt-level)
   both target a float-drift failure mode that **demonstrably does not
   exist here** (§2). They would solve a non-problem.
2. **The baselines are genuinely stale** w.r.t. a real, shipped,
   operator-reviewed UI change. Regenerating captures *exactly* that change
   and nothing else (verified: new-vs-old baselines differ **only** inside
   the sidebar — 0 content-area change baked in). This is the minimal,
   reviewable, correct delta.
3. **The strict-byte gate is working as designed.** It caught a real +1-row
   structural change. Loosening it to a per-pixel tolerance *now* would
   risk masking exactly this class of small structural change (e.g. a
   future stray nav item, a 1px layout nudge) — the opposite of what we
   want from a regression gate.
4. **It honors the determinism contract** rather than changing it. The
   bootstrap feature (`spec/ui-test-harness-bootstrap/feature.md`, H1 +
   falsifier lines 664–679) commits to strict-byte *because* tiny-skia CPU
   rasterization is deterministic, and says to switch to
   `matches_hash`-with-tolerance **only if** a non-deterministic diff
   appears. No such diff appeared — two consecutive runs produce
   byte-identical PNGs and zero `target/visual-diff/` output.

**Why NOT regenerate-only-without-thinking (the brief's anti-pattern):**
a blind re-baseline would have masked the structural change without
classifying it. We classified first (§1, §3): confirmed the *only* change
is the intended sidebar item and the *only* non-sidebar delta is zero, then
regenerated. Belt-and-suspenders: every new baseline was byte-diffed
against its predecessor with the sidebar masked → 0 content drift.

### Regeneration procedure (for the record / reproducibility)

The harness (`tests/fixtures/visual_diff.rs::matches_screenshot`)
auto-writes a baseline when the file is **missing** (first-run semantics)
and returns `Ok`. So:

```bash
rm -f crates/ui/tests/visual-baselines/*.png \
      crates/ui/tests/visual-baselines/render_snapshots/*.png
# Pass 1 — harness writes fresh opt-3 baselines (all green, first-run):
cargo test -p ui --test visual_snapshots --test render_snapshots --no-fail-fast
# Pass 2 — strict-byte verify against the fresh baselines (must be green,
# zero target/visual-diff/ output):
cargo test -p ui --test visual_snapshots --test render_snapshots --no-fail-fast
```

Watch recipe for the >2-min build/run:
```
watch -n 15 'tail -n 20 /tmp/visreg/run_regen.log'
```

---

## 5. Proof the gate still catches a real regression

Injected the **smallest meaningful regression** — a 1-LSB change to the
dark accent token in `theme.rs`:
`ACCENT.dark = rgb(0x6F,0xB6,0xAE)` → `rgb(0x70,0xB6,0xAE)` (one channel,
+1/255).

Result: **all 56 fixtures RED** (8 render_snapshots + 48 visual_snapshots).
Smallest detected diff (`charts_screen_dark_floor`): **1628 content-area
pixels at max-delta = 1**, in the chart marker/legend region
(`x[196..342]`, i.e. *not* the sidebar — a genuine content regression).

The change was then **reverted**; `theme.rs` is byte-identical to HEAD and
the suites are green again. The strict-byte gate detects a 1/255 color
shift on a real UI element — it has not been weakened.

---

## 6. Architect decision item (NOT triggered by this incident)

The brief asked me to flag, for an architect decision, any change to the
**visual-test-determinism contract** (the vendored `iced_tiny_skia` fork
was chosen for "CPU determinism on Apple Silicon"; a tolerance policy or a
profile pin touches that premise).

**I made no such change** — the fix is a straight re-baseline of a confirmed
real UI change, which the brief says does *not* need escalation.

For the architect's *future* consideration only (file, do not act):

- **Proposal:** add a small perceptual tolerance (e.g. max per-channel
  delta ≤ T, OR fraction-of-pixels-differing ≤ F) to
  `matches_screenshot`, as defense against the float-drift class that a
  *future* toolchain/LLVM bump could introduce on a *different* host.
- **Tradeoff / why I did NOT do it now:**
  - The triggering condition in the bootstrap falsifier (a
    non-deterministic diff) is **absent** — determinism currently holds
    byte-exactly (§2), so the premise for the strict-byte fork is intact.
  - A tolerance band risks **masking** the very structural-change class we
    just caught (this incident's +1-row nav shift; a future 1px nudge).
    Any tolerance must be tuned to stay *below* the smallest real
    structural change while *above* expected AA jitter — a calibration
    that should be an explicit, owned decision, not a unilateral
    ui-designer change.
  - It weakens the justification for maintaining the vendored
    `iced_tiny_skia` fork (operator-locked 2026-05-20 for CPU
    determinism).
- **Recommendation:** keep strict-byte until a real cross-host
  non-deterministic diff is observed (CI on non-Apple-Silicon, or a
  toolchain bump that perturbs rasterization). At that point, re-scope to
  `matches_hash`-with-tolerance per the bootstrap falsifier, with a
  calibrated threshold + a regression-detection proof (like §5) that the
  new threshold still fails on the smallest real structural change.

---

## Files changed

- `crates/ui/tests/visual-baselines/*.png` — 48 top-level baselines
  regenerated (sidebar-only delta).
- `crates/ui/tests/visual-baselines/render_snapshots/*.png` — 8 baselines
  regenerated (6 matrix + 2 legacy M1-B; sidebar-only delta).
- `spec/dev-notes/visual-regression-gate-stale-baselines-2026-06-08.md` —
  this note.

No source files changed. `theme.rs` byte-identical to HEAD (the §5 proof
was reverted). Anchors untouched (`verify_anchors.sh` 119/119 — baselines
are not in anchor scope).
