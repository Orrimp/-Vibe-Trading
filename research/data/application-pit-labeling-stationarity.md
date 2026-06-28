# Application — Point-in-Time, Labeling, Stationarity & Data Quality

*Decision doc for analyst + architect. Distilled from `research/data/knowledge.md`
(primary) and the 100-entry `research/data/papers.md` ledger (cited `data[N]`), with
the cross-topic `research/SYNTHESIS.md` roadmap. This is the **"what do we change in
the app"** layer for the point-in-time / data-quality / labeling / stationarity
strand. It does NOT add papers.*

> **Scope of this file:** point-in-time / as-of data (our `core::pit::PitSeries`,
> ADR-0058), data-revision/backfill leaks, crypto data-quality (wash trading, pump-
> and-dump, CEX>DEX, source provenance), outlier cleaning (real-extreme vs data-
> error), the sampling unit (why daily time bars), labeling (triple-barrier / meta-
> labeling / future-turning-point leakage / regime-dependent label noise), fractional
> differentiation & the stationarity-vs-memory tradeoff, stylized facts (asset-
> specific; Bitcoin's drifting Hurst), survivorship/coin-selection, and the intra-bar
> fill assumption. The leakage/CV and synthetic-data strands live in the two sibling
> files.

---

## 1. Summary of the research

This strand is about **the integrity and vintage of the inputs** — the part of the
pipeline that decides whether the gate is even looking at honest data.

**Point-in-time / look-ahead in the data layer**
- **Look-ahead is measurable, not theoretical.** A value must be stamped with *when
  it became known*, never a later-restated value; data revisions/backfills are a
  subtle leak `data[20]`. Look-ahead can even hide in a *model* (a pretrained LLM
  leaks the future through what it memorized — an open problem) `data[67]`.
- **Feature-store discipline names our consistency gates:** *point-in-time-correct
  retrieval* = our PIT/as-of rule; *training–serving skew* = our F5 forward-fidelity
  gap (bake-off ranks one implementation, forward runs a proxy) — fix is "one
  strategy definition, used everywhere." `data[39]`.

**Crypto data quality / provenance**
- **Crypto data quality is a first-order risk.** >70% of unregulated-exchange volume
  is wash-traded — **volume signals are suspect**; Benford / size-rounding are cheap
  data-quality gates `data[19]`. Pump-and-dump is frequent and concentrated in thin
  coins `data[82]`. **CEX price data is higher-quality than DEX** (efficiency <5bps
  vs 10–50bps; gas distorts DEX); source a reputable CEX/CEX-aggregate feed and stamp
  provenance `data[55][83]`.

**Outlier cleaning**
- **Two opposite kinds of "outlier":** data-error prints (instant-revert wicks, feed
  glitches → clean/remove) vs real extreme events (crashes → KEEP — they're the tail
  risk the gate exists to stress). Never silently winsorize a real crash bar.
  Field-standard cleaning = a *loose* robust (median/MAD, ~10-MAD) filter after
  deterministic checks (zero/neg prices, high<low, duplicate timestamps). `data[37][64][36]`.

**Sampling unit**
- **Time bars are statistically inferior** (oversample noise, undersample
  information) vs information-driven (tick/volume/dollar) bars — but we use **daily
  time bars**, which are operator-legible and *safer in crypto* because volume/dollar
  bars inherit wash-trade contamination `data[19]`. This is why our daily returns are
  non-normal + serially correlated (⇒ block bootstrap + non-normal metrics are
  right). `data[38]`.

**Labeling (only if we ever add ML/labels)**
- **Triple-barrier (vol-scaled barriers + time limit) > fixed-horizon returns** —
  path-aware, the crypto-proven stack pairs it with fractional differentiation
  `data[10][53][74]`. **Meta-labeling** (primary = direction/recall, secondary =
  act-or-not/precision) is an attractive *architecture lens even without ML* — our
  robustness gate is a "should we act?" filter in that spirit `data[53]`.
- **Labels are a leakage and noise surface.** Future-turning-point trend labels leak
  (a clever labeler showing 498% vs B&H is a red flag, not a result) `data[66]`;
  label noise is **non-stationary** — worst in exactly the volatile regimes a
  strategy most needs to be right `data[70]`; overlapping labels need uniqueness
  sample-weights + sequential bootstrap + purge/embargo `data[40]`.

**Stationarity & stylized facts**
- **Stationarity-vs-memory tradeoff:** integer differencing (returns) kills level
  memory; fractional differentiation keeps it at the minimum d for stationarity.
  **Crypto's trends live in the non-stationary level** — which explains why trend
  rules exist *and* why they're fragile (the level ≈ random walk). `data[41]`.
- **Stylized facts are ASSET-SPECIFIC, not universal** `data[42]`. Bitcoin's facts
  are strong but **drifting** — Hurst rose 0.42→0.49 (efficiency increasing toward a
  random walk), independent support that exploitable autocorrelation is *shrinking
  over time* `data[43]`. **Crypto volatility differs from equities** — inverse
  leverage effect (positive returns raise vol, sign-flipped), lower persistence, and
  jumps that dominate the tail `data[44]`.

**Survivorship / coin selection**
- Survivorship is a *universe-selection* problem; single-coin sidesteps the worst,
  but it reappears as *which coins we point at* (only-survives-today = conditioned on
  survivors) and *how we handle delistings/de-pegs mid-window* `data[7]`.

**The simulator is part of the data pipeline**
- Backtest-engine correctness hinges on the **intra-bar fill assumption** (which of a
  stop/target inside one candle fired first); an optimistic assumption silently
  inflates results, exactly like a mis-scaled cost. `data[81]`.

---

## 2. Possible solutions / what can be done with this research

1. **Lean on the PIT primitive we already have.** `core::pit::PitSeries` (ADR-0058)
   makes look-ahead on sidecar features **unrepresentable** — `as_of` is the only
   query method, returning the most-recent record at-or-before the query; a future-
   data join is a **compile error, not a runtime bug**. Extend/confirm that derived
   price features route through the same as-of discipline. `data[20][39]`.
2. **Honor the data-revision contract.** `crates/data/src/revision.rs` already
   write+verifies a REVISION.toml aggregate-SHA (ADR-0032) so a backtest on later-
   corrected OHLCV is detectable. This is the data-vintage leak `data[20]` made
   auditable — keep provenance stamped.
3. **A data-cleaning runbook.** Deterministic rejects (zero/neg, high<low, duplicate-
   timestamp merge, single-source provenance) → then a *loose* robust median/MAD
   (~10-MAD) outlier *flag for review*, never an auto-winsorize; trailing-only if it
   ever feeds a live decision. `data[64][37]`.
4. **Crypto data-quality gates.** Prefer reputable CEX/CEX-aggregate feeds; treat
   volume-based indicators as low-trust; optionally run Benford/size-rounding sanity
   checks on a new coin's history; warn on pump/wash artifacts. `data[19][55][82]`.
5. **Coin-selection guidance.** Favor large, liquid coins (BTC/ETH/SOL); flag thin-
   coin histories as manipulation-prone (the crypto microcap p-hacking trap); state
   that results are conditional on the coin surviving the window. `data[82][95][7]`.
6. **Document "why daily time bars"** and what it costs (statistically inferior but
   operator-legible and wash-trade-safe), and use it to justify block bootstrap +
   non-normal metrics over a plain Sharpe + i.i.d. bootstrap. `data[38][19]`.
7. **Close training–serving skew (F5).** Ensure the forward paper-trade runs the
   *exact* crowned strategy (reuse the bake-off's ComposedStrategy-from-TOML), not a
   proxy. `data[39]`.
8. **Make the intra-bar fill assumption explicit + tested.** For any strategy with
   intra-bar exits, assume the *adverse* level first (conservative) or resolve next-
   bar; pin it down with a test. `data[81]`.
9. **(Only if we ever add ML/labels)** triple-barrier (vol-scaled) + fractional
   differentiation; avoid future-turning-point labels; account for regime-dependent
   label noise; use the meta-labeling "should we act?" framing. `data[10][53][74][66][70][41]`.

---

## 3. Relevance for the project

- **It is the integrity floor under the gate.** The robustness machine can only be a
  credibility layer if the bars feeding it are honest. PIT discipline, data-revision
  detection, conservative outlier handling, and CEX provenance are what make the
  gate's verdict *trustworthy*, not just *computed*. This is the literal substrate of
  "traceable and plausible."
- **We already have the strongest primitive in the literature.** `PitSeries` turns
  look-ahead from a runtime bug into a compile error `data[20][39]` — most papers in
  this strand are *arguing for* the discipline we already enforce structurally. The
  work is to *confirm price/indicator features route through it*, not to invent it.
- **It explains and justifies our existing choices.** Daily time bars `data[38]`,
  block bootstrap over i.i.d. `data[13][90]`, non-normal metrics over plain Sharpe
  `data[25][90]`, large-liquid-coin scope `data[82][95]` — the research is the
  *rationale* for decisions already baked into the product, which we can now cite.
- **F5 (training–serving skew) is a known, named gap.** The forward paper-trade
  running an SMA proxy for non-SMA crowned picks `data[39]` is a real divergence
  between "the strategy we ranked" and "the strategy we watch" — fixing it is data-
  discipline, not a feature.
- **Honest on expected-null.** Bitcoin's rising Hurst `data[43]` is a quantitative
  "is there even an edge to find?" pre-check — it says exploitable autocorrelation is
  *shrinking*, reinforcing the thesis. Crypto's jump-dominated tails `data[44]` are
  why we preserve real extremes rather than smooth them. None of this is alpha; it is
  the discipline that keeps the verdict honest.

---

## 4. Advantages for the project

- **Auditability.** PIT as-of joins + REVISION.toml provenance + a documented
  cleaning runbook make every input reproducible and every revision detectable — the
  audit trail the product sells.
- **Robustness.** Conservative outlier handling (keep real crashes, remove only error
  prints) ensures the bootstrap stresses the *real* tail risk; a silent winsorizer
  would flatter every strategy and break the weakest-link verdict. `data[37][64]`.
- **Honesty.** Stating "results are conditional on the coin surviving the window" and
  "volume is partly wash-traded, so we down-weight volume signals" is measured honesty
  the operator can verify. `data[7][19]`.
- **Credibility via the right sampling unit.** Daily bars are defensible *and* wash-
  trade-safe in crypto; documenting why (vs volume/dollar bars) pre-empts a "why not
  information-driven bars?" critique. `data[38][19]`.

---

## 5. Problems and challenges

- **HARD CONSTRAINT — `ui` must NOT depend on strategy/exec/llm/models.** Surfacing
  data-quality warnings (wash/pump flags, provenance, "conditional on survival")
  in the cockpit must route through whatever DTO the UI already consumes, *not* by
  the `ui` crate reaching into strategy/exec/llm/models. Any new data-quality signal
  shown to the operator crosses this boundary and must be designed as a plain data
  field on the existing report/DTO surface.
- **HARD CONSTRAINT — Decimal not f64.** All price/OHLCV cleaning, MAD thresholds,
  and labeling barriers are financial quantities and must stay `Decimal`. The PIT
  layer is already Decimal-native (`PitSeries` payloads, `TimestampMs(i64)` key).
- **HARD CONSTRAINT — overlays ship a day-1 baseline-equity-divergence e2e.** If any
  data-quality decision becomes a *strategy/sizing modifier* (e.g. "skip trades on
  flagged-manipulation bars"), it is an overlay and inherits the day-1 divergence-e2e
  mandate (the v3-vol-overlay-noop precedent). A pure *display* warning does not; a
  *behavioral* filter does.
- **HARD CONSTRAINT — anchored report SHAs byte-immutable.** Adding data-quality
  fields to an anchored report mutates its body-SHA; land new fields in new
  files/fields or use the § D6.b re-emission protocol. Run `scripts/verify_anchors.sh`
  before AND after.
- **Outlier cleaning is genuinely ambiguous.** Distinguishing a manipulation-pump
  spike (real, happened, won't reliably repeat `data[82]`) from a market crash (real,
  tail risk to preserve `data[37]`) from a feed-glitch wick (error, remove) is not
  always clean. The loose-MAD-flag-for-review posture is deliberately conservative —
  it must *flag*, not auto-act, or it risks erasing the very tail the gate exists for.
- **F5 fix has real surface.** "One strategy definition everywhere" means the forward
  paper-trade must instantiate the exact ComposedStrategy the bake-off ranked — a
  non-trivial reuse, not a config tweak. `data[39]`.
- **Labeling/ML is a whole separate program.** Triple-barrier, frac-diff, uniqueness
  weights, sequential bootstrap, regime-dependent noise — all of this is *only*
  relevant if we add labels/ML, and it is large. For the current rule-based advisor it
  is **background to know, not work to do now.** `data[10][40][53][74]`.

---

## 6. Concrete next steps / candidate work items

**P0/P1 — F5: close the training–serving skew (one strategy definition everywhere).**
- **What:** the forward paper-trade must run the *exact* crowned strategy, not an SMA
  proxy. Reuse the bake-off's ComposedStrategy-from-TOML in the forward build path.
- **Where:** the forward-plan/build path that currently substitutes a proxy (the F5b
  fix referenced in project memory: "reuse the bake-off's ComposedStrategy-from-TOML
  in build_registry_for"). This is a correctness fix, not a new feature. `data[39]`.
- **Priority:** this is the most concrete, already-identified gap in this strand; the
  feature served must equal the feature evaluated, or the forward number is measuring
  a different strategy than the one we crowned.

**P1 — Leakage/PIT confirmation for price + indicator features.**
- **What:** confirm (with a test) that price/indicator features honor the same as-of
  discipline `PitSeries` enforces for sidecar features — trailing-only windows, next-
  bar fills, no use of a bar's own close before its fill bar.
- **Where:** `core::pit` (the primitive), the indicator/feature path, new tests under
  `crates/backtest/tests/`. `data[2][20][39]`.

**P1 — Data-cleaning runbook + deterministic-reject gate.**
- **What:** document and enforce: reject zero/neg prices, high<low, merge duplicate
  timestamps, single-source provenance; then a *loose* robust median/MAD (~10-MAD)
  outlier **flag for review** (never auto-winsorize). Trailing-only if it ever feeds a
  live decision.
- **Where:** `crates/data/src/` loaders (`binance.rs`, `coinbase.rs`, `kraken.rs`,
  `yahoo.rs`) + a shared cleaning module; provenance via the existing
  `crates/data/src/revision.rs` (ADR-0032). `data[64][37][19][55]`.

**P1 — Coin-selection + data-quality guidance (display, not behavior).**
- **What:** warn the operator when a chosen coin's history shows pump/wash artifacts
  or is thin/illiquid; state "results conditional on coin surviving the window." A
  *display* surface (no strategy behavior change ⇒ no overlay e2e mandate).
- **Where:** a plain data field on the existing report/DTO the `ui` already consumes
  (respecting the ui-dependency constraint). `data[82][19][95][7]`.

**P1 — Intra-bar fill assumption: make explicit + test.**
- **What:** for strategies with intra-bar exits (stop/take/triple-barrier-style
  levels), assume the *adverse* level first or resolve next-bar; pin it with a test.
- **Where:** the engine fill logic + a test analogous to the cost-model audit.
  `data[81]`.

**P2 — Document "why daily time bars" + Hurst/efficiency pre-check.**
- **What:** a docs note justifying daily time bars (operator-legible, wash-safe,
  explains non-normal/serially-correlated returns ⇒ block bootstrap + non-normal
  metrics); optionally surface the coin's Hurst/efficiency trend as an "is there even
  an edge?" pre-check. `data[38][43][15]`.

**P2 (only if ML/labels are ever added) — labeling stack.**
- **What:** triple-barrier (vol-scaled) + fractional differentiation; avoid future-
  turning-point labels; uniqueness weights + sequential bootstrap + purge/embargo for
  overlapping labels; meta-labeling "should we act?" framing — all still subject to
  the robustness gate + DSR/PBO. `data[10][40][53][74][66][70][41]`.

---

## 7. Open questions for analyst & architect

1. **F5 sequencing:** is the ComposedStrategy-from-TOML reuse a P0 (it's a
   correctness gap — the forward number currently measures a different strategy than
   the crown) or P1? It bumps no FROZEN constraint, so it's "just" engineering — but
   it is load-bearing for the honesty of the forward paper-trade.
2. **Cleaning policy ownership:** does the deterministic-reject + loose-MAD-flag
   cleaning live in the data loaders (per-source) or in a shared module the loaders
   call? And do we *flag-only* (safest) or ever *exclude* gappy/error windows from the
   gate input?
3. **Display vs behavior for data-quality warnings:** confirm the wash/pump/thin-coin
   warnings stay *display-only* (plain DTO field, no overlay e2e) and never become a
   silent trade filter (which would be an overlay + a hidden survivorship decision).
4. **UI boundary:** what is the exact DTO/report surface the `ui` already consumes,
   so a new data-quality field doesn't tempt a `ui → strategy/exec/llm/models`
   dependency?
5. **Coin-selection scope:** do we *restrict* the advisor to a liquid-coin allowlist,
   or *warn* on thin coins and let the operator proceed? The research favors a liquid
   scope; the product may favor warn-and-proceed for honesty.
6. **Delisting/de-peg handling:** what is the policy when a coin goes to zero or is
   delisted mid-window — truncate (a survivorship leak `data[7]`), carry to zero, or
   flag-and-stop? This needs an explicit, documented rule.

---

## 8. What NOT to do / effort & blast radius

- **Do NOT auto-winsorize outliers.** A silent winsorizer erases the real crash bars
  the bootstrap exists to stress and would flatter every strategy. Flag for review;
  remove only impossible prints (zero/neg, high<low, instant-revert wicks). `data[37][64]`.
- **Do NOT build a labeling/ML pipeline now.** Triple-barrier/frac-diff/meta-labeling
  is background for a *future* ML program, not work for the current rule-based
  advisor; building it now adds surface with no current consumer. `data[10][53][74]`.
- **Do NOT let a data-quality warning silently change trade behavior** without the
  day-1 baseline-divergence e2e — a behavioral filter is an overlay.
- **Do NOT trust volume/liquidity signals at face value** in crypto — wash trading
  makes them doubly suspect, and the liquidity anomaly family is the *least*
  replicable even in clean equity data. `data[19][95]`.
- **Effort / blast radius:** **F5** is medium effort, contained to the forward-build
  path, zero FROZEN-constraint impact — the highest-value item here. The **cleaning
  runbook** and **PIT confirmation** are low-to-medium effort, touching the data
  loaders and adding tests. **Data-quality display warnings** are low effort *if* kept
  to a plain DTO field (the ui-boundary is the only risk surface). The **labeling
  stack** is deliberately deferred (large, no current consumer).
