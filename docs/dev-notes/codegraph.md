# CodeGraph — code-knowledge-graph for AI agents (dev tooling)

[CodeGraph](https://github.com/colbymchenry/codegraph) is a pre-indexed knowledge
graph of this codebase (symbols, call paths, blast radius) that lets an AI coding
agent get **one-call** answers — "who calls X", "what breaks if I change Y", "show
me the relevant symbols + source for area Z" — instead of grepping and reading files
one at a time. It is a **developer/agent-navigation tool only**: it is NOT a Cargo
dependency, is NOT part of the trading product or runtime, and has **zero** effect on
builds, the test suite, the `verify_anchors` gate, or any shipped artifact.

Full Rust support (`.rs`, tree-sitter); this repo indexes at **723 files / 16,128
nodes / 54,680 edges** (6,381 functions, 656 structs, 252 enums, 26 traits).

## Setup (one-time, per machine)

```bash
# 1. Install the CLI (self-contained — bundles its own Node runtime).
npm i -g @colbymchenry/codegraph        # requires Node 22.5+ for the lib path; CLI bundles its own

# 2. Build the graph for this repo (creates .codegraph/, ~46 MB SQLite DB).
cd <repo> && codegraph init             # auto-sync is on by default; incremental thereafter
```

`.codegraph/` is a **local, per-machine artifact** — gitignored (root `.gitignore`),
never committed, rebuilt with `codegraph init`. It respects the repo `.gitignore`.

## CLI usage

```bash
codegraph status                 # index stats (files / nodes / edges by kind)
codegraph query <search>         # fuzzy symbol search
codegraph explore <query...>     # relevant symbols' source + call paths, one shot
codegraph node <name>            # one symbol's source + caller/callee trail
codegraph callers <symbol>       # who calls this function/method
codegraph callees <symbol>       # what this symbol calls
codegraph impact <symbol>        # blast radius — what a change to <symbol> affects
codegraph sync                   # manual incremental re-index (usually automatic)
```

Real examples on this codebase:

```text
$ codegraph impact rank_candidates
  Impact of changing "rank_candidates" — 27 affected symbols
  crates/backtest/src/bakeoff/rank.rs: rank_candidates, t62..t66 tests, …

$ codegraph callers classify_verdict
  Callers of "classify_verdict" (17): …
```

## Optional: wire it as an MCP server for Claude Code (OPT-IN — you choose)

This makes the `codegraph_*` tools callable by the agent directly (fewer grep/read
round-trips). It **registers a startup MCP server in the agent's config**, so it is
deliberately left as an explicit opt-in — it is NOT applied automatically by this repo.

**Project-scoped** (travels with the repo; prompts each user to trust it on open) —
create `.mcp.json` at the repo root:

```json
{
  "mcpServers": {
    "codegraph": { "type": "stdio", "command": "codegraph", "args": ["serve", "--mcp"] }
  }
}
```

**Global** (this machine only, all repos):

```bash
codegraph install --target claude --location global    # writes ~/.claude.json
# inspect first without writing:  codegraph install --print-config claude
```

Either way it takes effect on the **next** agent session (MCP servers load at start),
and Claude Code prompts to approve the server before it runs. Prerequisite: the
`codegraph` CLI must be installed (step 1 above) or the server fails to start
(harmlessly).

## Notes

- Scope `codegraph install` with `--target claude` so it does not touch other agents
  (Cursor/Codex/Gemini/etc.) it can auto-detect.
- The graph also picked up the 7 `scripts/*.py` and 1 Swift file — harmless; the
  723-file index is overwhelmingly the Rust tree.

---

## Calibration — measured 2026-08-15: caller lists are a LOWER BOUND

CodeGraph is applied and healthy on this repo (801 files · 18,945 nodes · 65,242 edges · 773 Rust +
20 Python; the daemon keeps it synced — `codegraph sync` reported *"Already up to date"* against a
tree edited minutes earlier, and the index contained both a file and a function created that day).
**Freshness is not the caveat. Completeness is.**

**The measurement.** Two calls inside `crates/agent/src/runtime.rs` — `short_exec::plan_open_short`
(`:2275`) and `short_exec::check_and_liquidate` (`:2616`) — are **absent** from `codegraph callers`,
while `grep` finds both. The same commands return correct, complete answers for the sibling call
sites in `crates/backtest/`. Scope was then bounded by experiment:

| probe | result |
|---|---|
| Is the index stale? | **No** — it contains that day's new file (`short_long_friction_parity_forward_e2e.rs`, 45 symbols) and new fn (`dvol_arm_compiled`, `bakeoff/mod.rs:194`). |
| Is `runtime.rs` excluded? | **No** — indexed, 71 symbols. |
| Does *any* caller in `runtime.rs` resolve? | **Yes** — a 4-space-indented call at `:1648` correctly resolves to enclosing fn `run` (`:817`). |
| Is it "very large functions"? | **No** — `run` also spans 800+ lines and resolves fine. |
| What do the two misses share? | Both sit ~24–40 spaces deep, inside nested `async`/closure blocks within `spawn_trading_loop` (`:1993-2692`). |

**The rule that follows:** treat `codegraph callers` as a **lower bound**, not a census. It is
excellent for *finding* call sites (that is its job, and it is far faster than grep at it). It is not
sufficient for *proving absence* — and "this symbol has no production callers" is exactly the claim
this project keeps needing, because the whole declared-vs-executed defect family (#65 → #91) turns on
reachability.

**The near-miss that motivated this note.** Bug-log **#90**'s census concluded that
`check_and_liquidate` has one caller per side — `agent` and `backtest` — and is therefore *symmetric*,
not a repeat of #80's asymmetry. Codegraph alone reports only the `backtest` caller. Had the census
been taken from it, #90 would have read "only the ranking side liquidates," which is **false**, and
the entry would have been wrong in precisely the way **#82** was wrong: asserting reachability without
tracing the caller graph. The grep cross-check is what prevented it.

**Working rule.** Lead with CodeGraph to orient and to find candidates. When a conclusion depends on a
caller set being *complete* — a reachability claim, a dead-code claim, a "no production callers"
claim — confirm with `grep -rn --include='*.rs'` before writing it down. The two tools disagree
exactly where this codebase's defects live: deep inside the async loops that execute real plans.

## Correction (2026-09-14) — the miss pattern is SYNTAX, not depth

The calibration above is right about the **rule** (caller lists are a lower bound; grep before any
completeness claim) and wrong about the **cause**. It attributed the two `runtime.rs` misses to calls
"~24–40 spaces deep inside nested `async`/closure blocks". Re-measured against `grep` across 11 call
sites, depth predicts nothing — a call at indent **28** resolves, calls at indent **4** are missed.
Two syntactic patterns account for every observed miss:

| pattern | example | codegraph |
|---|---|---|
| **callee named via a path that crosses a crate boundary** | `risk::size_portfolio_target(` — `agent/tests/v1_rebalance_reject.rs:88`, indent **4** | **missed** |
| … including a module imported from another crate | `short_exec::plan_open_short(` in `agent` (via `use backtest::short_exec;`) | **missed** |
| the same qualified call from a same-crate module | `short_exec::plan_open_short(` in `backtest/src/scenarios/sma_composed_run.rs`, indent **28** | found |
| a bare name brought in with `use`, even cross-crate | `spawn_aggregator(` in `ui/tests` (agent -> ui) | found |
| **any call inside a macro invocation** | `window.file_slug()` inside `format!(…)` — `reports/src/lib.rs:418`; `.slug()` inside `assert_eq!` | **missed** |
| the same method call outside a macro | `self.slug()`, `period.slug()` | found |

The original two misses (`short_exec::plan_open_short`, `short_exec::check_and_liquidate` in
`runtime.rs`) are both the cross-crate-path pattern — they happened to also be deep, which is the
confound. The `backtest` sibling calls that "resolved correctly" are same-crate calls.

**Why this is worse than a depth problem would be:** the missed form is where production callers
live. Observed misses include the ONLY production caller of `size_portfolio_target`
(`backtest/src/scenarios/montecarlo.rs:520`) — so codegraph reports it as having no production
caller, which is exactly bug-log #69's pre-fix state — plus `spawn_aggregator`'s
(`ui/src/bin/cockpit_live.rs:648`), `file_slug`'s (`reports/src/lib.rs:418`) and `run_path`'s bin
(`backtest/src/bin/param_robustness_sweep.rs:894`). A text census finds **~1,400** call sites of the
cross-crate-path form (**~580 under `src/`**), concentrated in `agent/src/runtime.rs` (65) and
`ui/src/bin/cockpit_live.rs` (63) — the code that executes plans. (Regex estimate; the pattern was
verified on 8 sampled sites, not all 1,400.)

Also observed: `codegraph_explore` (MCP) annotates `file_slug` with "no covering tests found" while
listing a test as its only caller, and a four-term query spent its output budget on an unrelated
`scripts/spec_lint.py` class. Treat its coverage and relevance hints as leads, not facts.

**The working rule above stands unchanged** — orient with CodeGraph, confirm completeness with grep.
What changes is that no call site is safe to trust on depth grounds: shallow, qualified calls are
missed too.

### Measured on 1.6.0, and pinned down with a minimal repro (2026-09-14)

**Upgrading does not help.** CodeGraph 1.6.0 (latest, 2026-08-26; this machine runs 1.1.0) was
installed into a scratch prefix, a clean clone of `c196e43` was indexed with it (806 files / 19,021
nodes), and the same call sites were re-checked. Both controls still resolve; all eight previously
missed sites are still missed. 1.6.0's resolver does build an exact crate-name map from the
workspace `Cargo.toml` (`resolution/frameworks/rust.js`, `getCargoWorkspaceCrateMap`), but it serves
`use` imports, not call paths — and its extractor still declares `callTypes: ['call_expression']`
only, so nothing inside a macro invocation is ever a call site.

**The exact rule**, isolated on a two-crate workspace with one control per factor. Every target
function was confirmed indexed with `codegraph query`, so each miss is resolution, not extraction:

| call in crate `b` | form | 1.6.0 |
|---|---|---|
| `target()` after `use a::target;` | bare import, cross-crate | found |
| `local2()` after `use inner::local2;` | bare import from an inline module | found |
| `filemod::file_local()` | same-crate module **in its own file** | found |
| `a::target()` | through another crate's path | **missed** |
| `util::helper()` after `use a::util;` | module imported from another crate | **missed** |
| `inner::local()` | same-crate **inline** `mod inner { … }` | **missed** |
| `format!("{}", target())` | bare name inside a macro | **missed** |
| `assert_eq!(target(), 1)` | bare name inside a macro | **missed** |

So a path-qualified call `m::f()` resolves **only** when `m` is a same-crate module that lives in its
own file; every other path, and every call inside a macro, is invisible to `callers` and `impact`.
(The table above says "a same-crate module path resolves" — true for file modules only.)

### A complete census: `scripts/callers.sh`

```bash
scripts/callers.sh <symbol> [path...]      # default: crates/ ; runs from any subdirectory
```

It runs `codegraph callers` plus a grep for call sites, and prints every grep site as ✓ (CodeGraph
knows the enclosing function) or ✗ (it does not), with a hint for why — `[path-qualified call]`,
`[inside a macro?]`, `[inside a string literal?]` — and a summary line, e.g.
`12 call site(s) · 7 in functions CodeGraph reports · 5 grep-only` for `size_portfolio_target`.

**Any "no callers" / dead-code / reachability claim goes through this, not through
`codegraph callers` alone.** ✗ lines are candidates to read, not proven callers: grep also matches
string literals and same-named methods on other types.

**Upstream:** no existing issue covers either gap (searched 2026-09-14; the analogous qualified-call
fixes exist for C++ #790, Go #1640 and Python #1704). Both are drafted from the synthetic repro above
and not yet filed.

### Census, 2026-09-15 — every unique-name function, against an independent ground truth

Reproduce with `scripts/codegraph_bench/run.sh`; add `LABEL=path/to/codegraph.db` to compare another
index side by side. Measured at `b9a15fc` on CodeGraph 1.1.0 (installed) and 1.6.0 (latest).

**Method.** The ground truth is a small Rust lexer (`scripts/codegraph_bench/gt.py`), independent of
both CodeGraph and grep. It skips comments, strings, char literals and `#[attributes]`, and tracks
brackets, so it knows each call's enclosing function and whether the call is inside a macro. The
population is every function whose name is defined exactly once in `crates/` and that has at least one
call: **2,448 functions, 11,790 call sites** (3,670 production, 8,120 test). Calls that cannot target
our function are excluded on meaning, not text: `x.f()` counts only if `f` is a method, bare `f()` only
if it is a free function, and paths through external crates are dropped (7,788 candidate sites).
CodeGraph's `calls` edges are read straight from its SQLite index; on five spot-checked symbols they
match `codegraph callers` exactly, apart from crate-root `pub use` re-exports, which the CLI also lists.
**Audit:** 20 of 20 randomly sampled misses are real calls. Of 8 sampled CodeGraph edges with no
matching ground-truth call, 7 are genuine false positives.

| metric | 1.1.0 | 1.6.0 |
|---|---|---|
| call-site recall, all code | **71.3%** | 71.2% |
| ... production code | **80.7%** | 80.3% |
| ... test code | 67.0% | 67.0% |
| caller-function recall | 73.1% | 73.0% |
| functions with ALL callers found | 73.3% | 73.0% |
| functions reported as having **zero** callers (they have at least one) | **9.3%** | 9.5% |
| functions whose **every** production caller is missed | **12.8%** | 13.1% |
| precision (the caller fn really makes a valid call) | 97.1% | 97.3% |
| `callers` latency p50 / p90 (n = 40) | 150 / 169 ms | 169 / 179 ms |

Call-site recall by how the call is written (1.1.0; 1.6.0 is within a point everywhere except
methods, 98.4%):

| call form | recall | sites |
|---|---|---|
| bare `f()` — local, or imported with `use` from the same or another crate | ~100% | 6,236 |
| `Type::f()` | 100% | 577 |
| method `x.f()` | 99.3% | 1,489 |
| `crate::...::f()` / `super::...::f()` | 96.2% | 80 |
| `m::f()`, `m` a same-crate file module | 64% | 11 |
| `m::f()`, `m` a same-crate module brought in by `use` | 14% | 193 |
| `m::f()`, `m` a module imported from another crate | 1% | 315 |
| `other_crate::f()` | **0%** | 745 |
| `m::f()`, `m` an inline `mod m { }` | 0% | 17 |
| any call **inside a macro** | **0%** | 2,053 |
| call in a `const` / `static` initializer | 0% | 66 |
| turbofish `f::<T>()` | 0% | 8 |

**What the numbers say.**
- **Two causes account for 98% of the misses.** 61% are macro-embedded — mostly test assertions:
  `assert_eq!` 780, `assert!` 363, `vec!` 248, `assert_snapshot!` 103, `format!` 89, `stream!` 47. Another 37% are calls written through a module or crate name. The remainder are
  `const`/`static` initializers, turbofish calls, and a handful of methods.
- **The rule, restated from the census:** CodeGraph resolves a call by the name at the call site.
  Bare names, methods and `Type::f()` are near-perfect, and full `crate::`/`super::` paths mostly work.
  A path through a module *alias* — a crate name, a module name brought in by `use`, or an inline
  module — almost never resolves. Nothing inside a macro is ever extracted. This supersedes the
  narrower "same-crate file module" phrasing earlier in this note.
- **The two `runtime.rs` misses that started this note have both causes.** They sit inside
  `tokio::select!` *and* go through the imported `short_exec::` module. Either one alone would hide
  them, which is why depth looked like the pattern.
- **False positives cluster on generic names.** External `tokio`/`reqwest` `.build()`,
  `Result::expect_err()`, `(1..n).all()` and `tempfile::tempdir()` get linked to a same-named workspace
  function. So `callers` and `impact` on a generically named function over-report as well as
  under-report.
- **Upgrading does not help.** 1.6.0 moves the path failures from "never extracted" to "extracted,
  unresolved" (its `unresolved_refs` table records them), but the outcome is identical. Indexing is
  deterministic: three builds of one commit gave identical edge sets, so re-running changes nothing
  until the code or the version does.

**Limitations.** Only functions with a workspace-unique name are measured (6,673 of 7,249 names), so
overloaded names like `new` are not covered. Receiver types are not resolved, so a few method sites
may be external calls with a colliding name; the audit found none among 20 sampled misses.
