# Codebase overview — what this system actually is, 2026-08-15

**Purpose.** The operator asked whether I had an overview of the code. I did not, and said so. This
is the artifact from actually building one. It is deliberately *not* a tour of the crates — see
§1 for why that would have been the wrong shape — and it should be read alongside its two
companion documents, which contain the raw evidence:

- [`reachability-map-2026-08-15.md`](reachability-map-2026-08-15.md) — entry points, the
  feature-propagation matrix, dead/unreachable inventory, config-to-consumer chains, forked paths.
- [`claims-ledger-2026-08-15.md`](claims-ledger-2026-08-15.md) — every capability and result claim
  in the docs, checked against the code, in both directions.

---

## 1. The method, and why the obvious one was wrong

My first instinct was to read the codebase crate by crate. That would have been a mistake, and the
reason is worth stating because it generalises.

The 14-story review burn-down that preceded this produced **nine** defect disclosures (bug-log
#74–#82). **Not one of them was a wrong algorithm.** Every single one was a *connectivity* failure:

| defect | what was actually wrong |
|---|---|
| #74 | a test injected the signal through a channel production does not use |
| #75 | one field served two purposes; the second write clobbered the first |
| #76 | a rank direction inverted relative to its own spec |
| #77 | a baseline regenerated from the code it was guarding |
| #78 | an arm in the ranked field that had not run |
| #79 | a config value that never reached the engine |
| #80 | one execution path bypassing the shared one |
| #81 | a module never compiled into any shipped build |
| #82 | a capability in a strategy's name that never happens |

Reading teaches you what each file *does*. It teaches you nothing about what is *connected*. Every
file in this repo is individually defensible; the defects live in the relationships between them.

So the method here was: **map reachability, then check claims against it.** Three subsequent
disclosures (#83, #84, #85) and ~22 further candidates came out of that in a few hours — including
one, #84, that had silently disabled the project's hardest invariant since CI was activated.

### 1a. Why the compiler cannot help you here

`cargo check --workspace --all-targets` emits **zero** dead-code warnings. That is not a clean bill
of health. **rustc does not report unused `pub` items in a library crate** — it assumes a downstream
consumer. In a 17-crate workspace where nearly everything is `pub` to cross a crate boundary, the
language's own detector for this exact defect class is blind by construction. Every Tier-1
reachability finding is `pub`.

That is the structural reason nine connectivity defects shipped past green builds, and it means
grep-level identifier-occurrence counting and write-site-vs-read-site separation are not fallbacks
in this repo — they are the only techniques that work.

**The sharpest single discriminator found**, worth adopting as a rule: for a `cfg(feature = …)`
gated capability, the question is not "is the feature enabled?" but **"does the `cfg(not(...))` arm
`bail!`, or does it return a plausible value?"** `backtest/candle` is enabled nowhere and is nearly
harmless, because all three of its off-arms bail with the rebuild command. `backtest/realdata`
returns a bare `None` — and that difference is the entire severity gap between a non-issue and #81.

---

## 2. Size and shape

| | files | lines |
|---|---|---|
| production (`src/`, excluding inline `#[cfg(test)]`) | 405 | ~153,000 |
| tests (`tests/` + inline) | 353 | ~169,000 |
| **total** | 772 | ~322,000 |

17 crates. `ui` is by far the largest (249 files); then `backtest` (89), `reports` (54),
`strategy` (52), `data` (47), `audit` (45), `llm` (43), `agent` (40).

**There is more test code than production code**, and the burn-down established that a substantial
fraction of it is vacuous — gates that cannot fail. So the test suite is not a trustworthy source of
intent: learning the contracts from it would teach you things that are not true. Read tests here as
*claims*, not as specifications.

---

## 3. What actually runs

The single most useful thing I learned. **In the shipped cockpit build, `backtest` compiles with no
features at all.** `ui` declares `backtest = { path = "../backtest" }` with no `features`, its whole
`[features]` section never mentions `backtest`, `backtest` has no `default` stanza, and neither
documented run command passes anything through. Consequences:

- `backtest/yahoo` off → the macro-regime loader is never compiled (**#81**).
- `backtest/realdata` off → `dvol_data`, `basis_data`, `funding_data` and `resolve_dvol_override`
  are never compiled; the last returns a bare `None` (**#81 extension**).
- ~5,310 lines of `backtest/src/` compile into **zero** shipping builds and zero CI runs.

The trap that hides this: `required-features = ["realdata"]` on backtest's own **bins** reads like
the feature is in use. It gates those binaries and propagates nothing to library consumers.

---

## 4. The parts I now trust, and the parts I do not

Both columns matter. A map that only lists faults is as misleading as one that lists none.

**Solid, verified:**
- **The PIT / as-of layer** — private `AsOf` fields, a single constructor, a trybuild compile-fail
  test, and a shape-matching lint with a self-test that plants a violation. The best-engineered
  gate in the repo.
- **The anchor gate itself** — 119/119, byte-immutable, and it genuinely binds locally.
- **The frozen gate's arithmetic** — band structure, weakest-link/all-pass logic, and the drawdown
  **units** (a documented fraction against a 0.70 threshold; no repeat of the two 100× bugs).
- **The NaN guard** in `reduce_samples` — correct, and well-reasoned.
- **The market-calendar layer** (3-16's deliverable A) — real holiday arithmetic, proven inert on
  the existing corpus.
- **No venue-write path exists — PROVEN 2026-08-15, five independent layers.** This is the single
  most safety-critical claim in the repo (it underwrites the operator's standing no-live-trading
  constraint), so it is no longer asserted, it is demonstrated:
  1. **Zero order endpoints** — no `/api/v3/order`, `/fapi/v1/order`, `/v5/order`, `newOrder`,
     `placeOrder`, `cancelOrder` string anywhere in production `src/`.
  2. **Zero request signing** — no HMAC, no `api_secret`, no `X-MBX-APIKEY`. Every "signature" hit in
     the tree is a Rust *function* signature in a doc comment; every `Sha256` is anchor hashing.
     Without signing, authenticated order placement is impossible on every major venue.
  3. **The only HTTP writes in the entire production tree are three LLM providers** — Anthropic,
     OpenAI, Ollama. Every other `.post(`/`.put(` hit is an LRU cache `put`.
  4. **The `exec` crate cannot reach a network at all, and Cargo enforces it.** Its `[dependencies]`
     contain no `reqwest`, no `tungstenite`, no `hyper` — only `trading_core`, `reflection`, and
     plumbing. This is the strongest layer: it is a *manifest* guarantee, not a code-discipline one.
  5. **The sole `ExecRouter` implementor workspace-wide is `PaperExecRouter`**, and it returns
     `Err("PaperExecRouter not yet wired (T24)")` — even the paper router is an unwired stub.

  **The tripwire, for whoever audits this next:** the guarantee breaks the moment a network client
  appears in `crates/exec/Cargo.toml`. That single line is the thing to watch — it is far cheaper to
  monitor than any amount of code review, and it is the reason layer 4 matters more than layers 1-3.
  Network capability in this workspace is confined to four crates (`agent`, `data`, `llm`, `ui`), and
  `exec` is deliberately not among them.

**Not trustworthy without checking:**
- Any claim that a gate *enforces* something — three separate stories shipped a mandated AD-16
  divergence gate that could not fail, and AD-16's meta-enforcement is a substring grep over test
  filenames.
- Any count shown to the operator — two were wrong (#81, #82's arm count).
- Any config value described as a limit — see #79 and #85.
- The CHANGELOG's per-feature result claims — several were falsified during the burn-down and are
  now marked in place.

---

## 5. The through-line

Every defect in this codebase, without exception so far, is the same sentence: **something is
declared in one place and not executed in another, and nothing compares the two.** A config field
and its consumer. A feature and its enabler. A strategy's name and its behaviour. A test's name and
its assertion. A doc's claim and its code.

The project has unusually strong machinery for *freezing* things — 119 byte-immutable anchors, a
frozen decision gate, an ADR corpus with atomic registration, a story↔trace↔CHANGELOG triad lint.
What it lacks is machinery for *connecting* things: nothing checks that a declared capability is
reachable, that a configured limit is read, or that a named gate can fail.

That is the gap worth closing, and it is closeable — #84 was a one-line fix, #81's honesty half was
a guard mirroring one that already existed, and the arm-count defect was a predicate that needed a
second match arm.

---

## 6. What I still do not know

Stated plainly, because the point of this document is to be honest about coverage:

- I have **never seen this cockpit render.** Every UI conclusion is via harnesses.
- The **full workspace test suite has never completed** in my hands.
- **CI has been red throughout** and its logs need repo-admin to read (403).
- I have read essentially **none** of `reports`, `llm`, `cost`, `forecast`, `trader`, `exec`,
  `risk`, and only fragments of `ui` (249 files), `audit` and `agent`.
- The 900-paper `research/` tree and the vendored `iced_tiny_skia` fork are entirely unexamined.
- The reachability sweep's findings were gathered while the tree was moving under it; items
  verified early were not all re-confirmed against the final state.

---

## 7. If you continue this

In rough value order:

1. ~~**Close the `cfg(not(...))` audit**~~ — **DONE 2026-08-15.** All 24 features carry a verdict.
   The last open candidate (NEW-B) resolved *against* its own alarming reading: the shipped cockpit
   **cannot** report an empty run as success (`rt_handle` is not an `Option`), and no build compiles
   the remaining `cfg` arm. Logged as **#91** at LOW, where the surviving finding is dated — the
   stub's rationale landed 2026-05-24 and `live` became default 2026-05-25. Also re-confirmed **#81
   against cargo's own resolver** rather than the manifests: `backtest` resolves to the empty
   implicit `default` in the CI cockpit build, and all three dependency edges declare it
   feature-less.
2. **Decide #83** — the frozen gate fails open. Needs an AD-1 ruling, not a patch, and the durable
   half is making `is_eligible` an allow-list rather than a deny-list.
3. **Wire or delete #85's loss stops.** Declared-but-inert is the worst of the three states.
4. ~~**Finish #80**~~ — **DONE 2026-08-15.** The forward paper loop now routes short legs through the
   engine, gated by a non-vacuous parity e2e in `crates/agent` (the gate had to move crates: the
   `backtest` one structurally cannot observe that loop). `try_open_short`/`try_cover_short` are now
   production-dead workspace-wide. The census that proved it surfaced a *third* exit from the same
   state — `check_and_liquidate`, invisible to both gates because it emits no `Fill` — logged as **#90**
   for an operator decision. Closing a fork does not close a family.
5. **1-25** remains the bottleneck for the research record: 20 anchored surfaces, four routed
   defects, blocked on an architect seam decision and a compute budget.
