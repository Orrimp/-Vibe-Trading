---
slug: lab-yahoo-empty-range-ux
status: draft
owner: analyst
updated: 2026-05-30
version: 0.1.0
---

# lab-yahoo-empty-range-ux — v0.1.0

Surface a clean, actionable operator message when a Yahoo fetch returns
**no data for the requested window**, instead of the current confusing
empty/hanging result that the operator cannot distinguish from "broken."

> **One-line:** distinguish "Yahoo has no data for this range" (an
> expected, non-error outcome — future-dated range or delisted ticker)
> from a real fetch failure (network / rate-limit / malformed response),
> and render a distinct, plain-language Lab status message for the former.

## Motivation (operator friction — 2026-05-29)

During Bug #64 D.1.1 verification, the operator repeatedly hit confusion
with the **Last 30d / Last 90d** Yahoo presets. The system clock in this
test environment is **2026-05-29** (future-dated), so `Last 30d` computes
`2026-04-29 → 2026-05-29` (`range_to_ms_pair`, `runner.rs:323-334`,
using `OffsetDateTime::now_utc()`). The real Yahoo Finance API has **no
data for those future-dated ranges**. The CLI fetch returned 0 rows in
~2 s with no parquets written; the cockpit showed a spinning/empty result
with no clear "there's nothing here" signal. The operator could not tell
**"broken" from "no data exists."**

This is the **FYI follow-up** the Bug #64 attempt-3 presenter deck
explicitly carried forward (see
[`spec/bug-64-d11-attempt-3-yahoo-run-runtime-context/presentations/bug-64-attempt-3-2026-05-29.md`](../bug-64-d11-attempt-3-yahoo-run-runtime-context/presentations/bug-64-attempt-3-2026-05-29.md)
§ Notes/feedback FYI #2: _"Last30d/Last90d show no data under the 2026
system clock — candidate for a future lab-yahoo UX-polish feature ('no
data for range' surfacing instead of a confusing empty result)."_).

This is **NOT** the Bug #64 reactor-context bug (that is CLOSED). This is
a separate, narrow UX gap on the empty-Yahoo-response path.

## Root-cause analysis (READ-ONLY code trace)

Confirmed the exact empty-response path by reading `crates/data/src/yahoo.rs`
and `crates/ui/src/lab/runner.rs`:

1. **`range_to_ms_pair`** (`runner.rs:323`) computes a future-dated
   `(start_ms, end_ms)` for `Last30d`/`Last90d` via `now_utc()`.
2. **`preload_yahoo_bars`** (`runner.rs:363`) calls `src.load_cached(...)`
   → cold cache → `YahooError::CacheMiss` → auto-fetch fallback
   (`fetch_with_backoff` → `fetch_and_cache`).
3. **`fetch_and_cache`** (`yahoo.rs:364`) calls Yahoo, gets **0 quotes**;
   `quotes_to_bars` returns an **empty `Vec`**; `write_bars_by_month`
   writes **nothing** (the group map is empty); `regenerate_revision_manifest`
   runs; then it round-trips `load_cached` again.
4. **`load_cached`** (`yahoo.rs:249`) now finds the requested
   `(year, month)` parquet **still absent** → returns
   `YahooError::CacheMiss` again **OR**, if a partial/empty file exists,
   the 95% coverage check (`yahoo.rs:321-335`) fails with
   `YahooError::MissingData { actual: 0, expected: N, pct: 0.0 }`.
5. **`preload_yahoo_bars`** wraps whichever error in a generic string
   (`"yahoo cache load (post-fetch): {e}"` or `"Yahoo auto-fetch failed
   for {ticker}: {e}. Check network connectivity..."`) and returns
   `Err(SmolStr)`.
6. The Lab screen (`screens/lab.rs:474-478`, Bug #54 wiring) renders that
   string verbatim with a `⚠` prefix in red (`DOWN_500`).

**The gap:** the empty-future-range case is presented identically to a
genuine failure — a red `⚠` error referencing a `CacheMiss`/`MissingData`
internal error and a misleading _"Check network connectivity"_ hint, even
though nothing is actually broken. The operator gets an error-styled
message that says "check your network" when the truth is "Yahoo simply has
no bars for a future-dated window."

### Why this is small

The detection point is a **single function** (`preload_yahoo_bars`,
`runner.rs:363-438`). The render point is a **single existing widget**
(`screens/lab.rs:474-478`). No new channel, no new Message variant
required for the minimum-viable version (the message can ride the existing
`last_run_error: Option<SmolStr>` field, or a sibling
`last_run_notice` if Q3 picks the distinct-styling path). Estimated
**~1–2 dev-days**.

---

## Requirements

### R1 — Detect the empty-result case at the preload boundary

`preload_yahoo_bars` (or a helper it calls) MUST distinguish:

- **(a) "Yahoo has no data for this range"** — a clean, *expected*
  outcome. Triggered when the post-fetch path yields **0 bars** (or
  coverage below threshold with `actual == 0`) AND the fetch itself
  succeeded (HTTP 200, no rate-limit, no parse error). Causes: a
  future-dated range relative to the latest available Yahoo bar, or a
  delisted/never-listed ticker for that window.
- **(b) An actual fetch error** — the existing path. Network failure,
  HTTP 429 (`RateLimited`), timeout, malformed response (`Parquet`/`Http`),
  revision tampering (`RevisionMismatch`). These remain red `⚠` errors.

The distinction MUST be derived from the *fetch outcome*, not guessed
after the fact (see Q1 for the heuristic-vs-explicit decision).

### R2 — Surface a distinct operator-facing "no data" message

When R1 detects case (a), the Lab UI MUST show a plain-language,
non-alarming message naming the ticker and the resolved date window, e.g.:

> No Yahoo data for SOL-USD in 2026-04-29..2026-05-29 (range may be
> future-dated or the ticker may be delisted).

It MUST NOT:
- reference an internal error variant name (`CacheMiss`, `MissingData`),
- include the misleading _"Check network connectivity"_ hint,
- be styled as a hard error if Q3 selects the distinct-notice path
  (otherwise it remains the `⚠` surface but with corrected copy).

The message MUST resolve and display the **actual computed window**
(post-`range_to_ms_pair`), since the future-dating is the operator's
primary confusion source.

### R3 — Run terminates cleanly (no hang, no spinner stuck)

The empty-data path MUST resolve the in-flight run to a terminal state
(`lab_run_inflight = false`, spinner cleared) within the existing
preload timeout budget — i.e. it MUST NOT leave the cockpit spinning. The
"no data" outcome is a **completed** run with zero bars, not an
indefinitely-pending one. (Today the run does terminate via the wrapped
`Err`; this requirement pins that the new path preserves termination.)

### R4 — Date-preset guard for future-dated ranges (scope set by Q2)

Depending on Q2, EITHER:
- **clamp** `Last30d`/`Last90d` so the computed `end_ms` never exceeds the
  latest available Yahoo bar (or `min(now, last_available)`), OR
- **warn** (pre-run) when the computed range extends past the latest
  available bar, OR
- **leave the presets as-is** and rely solely on R1/R2's post-run message.

Whichever Q2 selects, the chosen behaviour MUST be deterministic and
testable.

### R-NR — Non-regression

- **R-NR.1** Synthetic-data runs are byte-identical (this feature only
  touches the `data_source == YahooCache` preload branch and message
  rendering). Zero anchor delta.
- **R-NR.2** Genuine Yahoo fetch errors (network, 429, parse, revision)
  still surface as red `⚠` errors with the existing copy — case (b) is
  untouched in styling and routing.
- **R-NR.3** Warm-cache Yahoo runs with real data (e.g. `2024 H1`,
  `2024 H2`) are unaffected — they return bars and never hit the
  empty-data branch.
- **R-NR.4** At most one new operator-facing string constant family in
  `crates/ui/src/strings.rs` (the no-data template); no new design tokens.
- **R-NR.5** No change to the `YahooError` enum's existing variants'
  Display strings (those are referenced in `data` crate tests and the CLI).
  A *new* variant or a new classification helper is permitted; mutating
  existing variant messages is out of scope.
- **R-NR.6** Baseline-equity-divergence e2e gate is **N/A** — this feature
  ships no strategy overlay or sizing modifier (it is a data-availability
  UX path). The CLAUDE.md overlay-e2e non-negotiable does not apply; the
  required gate here is the empty-vs-error classification test (K2 below).

---

## Risks & mitigations (K)

### K1 — Mis-classifying a real failure as "no data"

If R1's heuristic is "0 bars ⇒ no-data", a *silent* fetch failure that
returns 0 bars without erroring (e.g. Yahoo returns HTTP 200 with an empty
body during an outage) would be mis-labeled as "no data" and the operator
would not be prompted to retry.

- **Mitigation:** Q1 biases toward the **explicit** signal — only classify
  as "no data" when the fetch path returned HTTP success AND the response
  was well-formed AND contained zero quotes. Any transport/parse/rate-limit
  error stays case (b). The classification is on the *fetch result type*,
  not solely on the final bar count.
- **Falsifier:** K2 test must include a "0 bars from a 200 response" case
  (→ no-data) AND a "transport error" case (→ error) AND assert they route
  to different surfaces.

### K2 — No test catches the empty path (the v3-vol-overlay precedent)

Per the CLAUDE.md non-negotiable lineage, a math/unit test alone is
insufficient. The empty-data classification must be covered by a test that
exercises the *preload boundary* with a faked Yahoo source returning zero
bars and asserts the resulting message is the no-data notice, not the
generic error.

- **Mitigation:** an injectable-source test using the existing
  `LabYahooBarSource` trait + `MockLabYahooBarSource` harness
  (`runner.rs:222`, ADR-0048) — the mock returns `Ok((vec![], sha))` (or
  the analogous empty signal) and the test asserts the classification +
  message. This reuses the Bug #64 callthrough test harness, no new infra.

### K3 — Clamp changes determinism / breaks a real range (Q2=(a) only)

If Q2 selects clamping, clamping `end_ms` to `now` or `last_available`
could subtly change which bars a run loads, risking a behaviour shift on a
range that previously errored.

- **Mitigation:** clamping (if chosen) applies ONLY when the original
  `end_ms > now_ms` (future-dated); past-and-present ranges are passed
  through unchanged. The "latest available bar" probe (if used) is bounded
  and cached, never a blocking network call on the render path.
- **Falsifier:** a clamp test asserting a non-future range is byte-identical
  pre/post.

### K4 — "Latest available bar" is unknowable without a fetch

Determining the true last-available Yahoo bar for a ticker generally
requires a network call, which we cannot do on the synchronous preset-build
/ render path.

- **Mitigation:** prefer the cheap, deterministic proxy `now_utc()` as the
  clamp/warn boundary (future = `end_ms > now_ms`). The richer
  "last-available-bar" probe is explicitly **deferred to v0.2.0+** if the
  operator finds the `now`-based boundary insufficient. This keeps v0.1.0
  free of a blocking probe.

---

## Hypotheses (H)

- **H1 — The empty path today always lands on `CacheMiss` or
  `MissingData`, never a silent success.** Code trace says the post-fetch
  `load_cached` re-check errors (no parquet written for an empty fetch).
  *Test:* the mock-empty preload test confirms the classification fires on
  both the `CacheMiss`-re-check and the `MissingData{actual:0}` shapes.
- **H2 — A new operator string + the existing `last_run_error`/notice
  field is sufficient; no new Message variant needed for v0.1.0.** *Test:*
  the implementation lands without adding a `Message` enum variant (if Q3 =
  reuse-existing-surface) — falsified if a new variant proves unavoidable.
- **H3 — `now_utc()` is a good-enough future-dating boundary for v0.1.0.**
  The operator's confusion is specifically about future-dated ranges under
  the 2026 clock; `end_ms > now_ms` captures 100% of that case. *Test:*
  a unit test on `Last30d`/`Last90d` under a pinned future clock asserts
  the range is flagged future-dated.

---

## Operator-decide questions

> All options framed **durable-over-quick** per AGENT.md 2026-05-28 — the
> `(Recommended)` tag is on the most durable choice, with an
> if-budget-tightens fallback named.

### Q1 — Empty-vs-error classification mechanism

How do we decide a result is "no data" vs a real failure?

- **(a) Explicit fetch-outcome classification (Recommended — DURABLE).**
  Thread the fetch result type through so "0 quotes from an HTTP-200,
  well-formed response" is a distinct outcome from any transport/parse/429
  error. Likely a new `YahooError::NoDataForRange { ticker, start_label,
  end_label }` variant (or a `LoadedBars { loaded_count: 0 }` success that
  the preload classifies), emitted ONLY when the fetch succeeded and
  returned zero usable quotes. *Durable:* correct under Yahoo outages
  (K1), extends cleanly to v0.2.0 equities (weekends/holidays produce
  legitimately sparse ranges), and gives the tester a typed thing to
  assert. Cost: ~1.5 dev-days (touches `data` crate classification +
  `runner` mapping + `ui` render).
- **(b) Bar-count heuristic — fallback if budget tightens.** Treat
  `actual == 0` (or coverage `pct == 0.0`) at the preload layer as
  "no data" purely from the count, without threading fetch-outcome type.
  Cheaper (~0.5 dev-day, `runner`-only) but *fragile* under K1 (a silent
  HTTP-200-empty outage looks identical to a real outage that returns 0
  bars) and spawns a v0.2.0 "make classification explicit" cleanup brief.
  Pick only if v0.1.0 must land in <1 day.

### Q2 — Date-preset guard behaviour (R4)

What do we do about `Last30d`/`Last90d` resolving to future-dated windows?

- **(a) Clamp the computed `end_ms` to `now_utc()` when future-dated
  (Recommended — DURABLE).** `Last30d` under a 2026-05-29 clock becomes
  `2026-04-29 → 2026-05-29` clamped to `… → min(end, now)`. Combined with
  R1/R2, the operator gets *either* real recent data (when the clock is
  real) *or* a clear no-data notice (when no bars exist even up to now).
  *Durable:* the presets do what they say ("last 30 days") regardless of a
  skewed clock, and the behaviour is identical when the clock is real (no
  regression). Cost: ~0.25 dev-day on top of R1/R2 (a `min` + a clamp
  test). **NOTE:** clamp applies only when `end_ms > now_ms` (K3).
- **(b) Pre-run warn, no clamp — middle fallback.** Render a one-line
  caution next to the range chip when the computed range extends past
  `now` ("range extends past today; Yahoo may have no data"). Leaves the
  range untouched. Cheaper to reason about but adds a second message
  surface and still produces the empty run.
- **(c) Leave presets as-is; rely solely on R1/R2 message — minimum
  fallback.** No preset change at all; the post-run no-data notice is the
  only signal. Smallest blast radius (~0 extra LoC) but the operator still
  has to *run* to discover there's no data. Pick only if Q1 already
  consumed the budget.

### Q3 — Message placement & styling (R2)

Where/how does the no-data message render?

- **(a) Reuse the existing run-button-row surface with a distinct
  *notice* style (Recommended — DURABLE).** Add a sibling
  `last_run_notice: Option<SmolStr>` (or reuse `last_run_error` with a
  notice/error flag) rendered in a neutral/info color (e.g. `FG_2` /
  muted), NOT the red `⚠ DOWN_500` error treatment. *Durable:* the
  operator visually distinguishes "no data (expected)" from "broken
  (act now)" — the core friction. Reuses the existing render site
  (`screens/lab.rs:474-478`), no new layout. Cost: ~0.5 dev-day (one
  field + one string + one style branch). The ui-designer confirms the
  notice token in the M-DEV-UI lane.
- **(b) Reuse `last_run_error` verbatim with corrected copy — fallback.**
  Keep the single red `⚠` surface but swap the misleading copy for the
  clear no-data sentence. Cheapest (no new field/style) but keeps the
  alarming red styling for a non-error outcome — partially solves the
  friction (copy fixed, affect not). Pick if Q1+Q2 consumed the budget.
- **(c) Toast / inline banner — rejected for v0.1.0.** A toast or
  dedicated banner is a larger surface change (toast queue wiring) than
  this small feature warrants. Out of scope; note for v0.2.0 if the inline
  notice proves too subtle.

---

## Verdict tree (4-cell, tester-facing)

| Outcome | Synthetic byte-identical (R-NR.1) | Empty-future-range → no-data notice (R1/R2) | Real fetch error → red ⚠ error (R-NR.2) | Verdict |
|---|---|---|---|---|
| **PASS** | ✅ anchors byte-identical | ✅ mock-empty preload renders no-data notice, names window, no "check network" hint | ✅ mock transport error renders red ⚠ error | **PASS** → presenter |
| **PARTIAL** | ✅ | ✅ | ⚠ error path copy regressed but routing intact | route to developer (copy fix) |
| **FAIL** | ✅ | ❌ empty range still shows generic/red error or hangs | — | route to developer (R1/R2 not met) |
| **REGRESSION** | ❌ anchor drift OR synthetic path changed | — | — | **block ship** — route to architect (R-NR.1 broken) |

---

## Out of scope (v0.2.0+ candidates)

- True "latest-available-bar" network probe for richer clamping (K4 defers
  this; `now_utc()` boundary ships at v0.1.0).
- Toast / dedicated banner surface (Q3=(c) rejected for v0.1.0).
- Equities market-calendar-aware sparse-range handling (weekends/holidays)
  — lands with the v0.2.0 multi-asset expansion already noted in
  `crates/data/src/yahoo.rs` `MISSING_DATA_THRESHOLD_PCT` doc comment.

## References

- `crates/ui/src/lab/runner.rs:323-438` — `range_to_ms_pair`,
  `preload_yahoo_bars`, `fetch_with_backoff` (detection point).
- `crates/data/src/yahoo.rs:119-188` — `YahooError` variants;
  `:249-351` `load_cached` (95% coverage check); `:364-413`
  `fetch_and_cache` (empty-response path).
- `crates/ui/src/screens/lab.rs:470-479` — Bug #54 error-render site
  (proposed render point).
- `crates/ui/src/lab/state.rs:197-205` — `last_run_error` field.
- `spec/dev-notes/bug-64-yahoo-run-code-map-2026-05-29.md` — state table
  + YahooError variant map + cancellation/progress diagrams.
- `spec/bug-64-d11-attempt-3-yahoo-run-runtime-context/presentations/bug-64-attempt-3-2026-05-29.md`
  § Notes/feedback FYI #2 — the carry-forward this feature discharges.

## Changelog

- 2026-05-30 (analyst): M0 — feature.md authored from the Bug #64
  attempt-3 deck FYI #2 carry-forward. READ-ONLY code trace pinned the
  empty-future-range path (`fetch_and_cache` 0 quotes → re-`load_cached`
  → `CacheMiss`/`MissingData` → generic red error). R1–R4 + R-NR,
  K1–K4, H1–H3, Q1–Q3 (all biased durable-over-quick), 4-cell verdict
  tree. New trace row `REQ-LAB-YAHOO-EMPTY-RANGE-UX-001` (proposed).
  HANDOFF → architect (M-T1 design pass).
